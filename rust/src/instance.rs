//! A node's instance of a `RubyScript`, handed to Godot through the
//! extension interface directly rather than gdext's `ScriptInstance`: Godot
//! deletes an instance the moment its node's script is set, even while a
//! call into it is running, and gdext keeps the instance borrowed for the
//! whole call. Each callback here borrows the instance only until Ruby is
//! about to run, and touches nothing of it afterwards, as GDScript, C# and
//! other languages' instances do.

use std::collections::BTreeSet;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use godot::classes::{ClassDb, Object, Script, ScriptLanguage};
use godot::meta::conv::RawPtr;
use godot::obj::{EngineBitfield, EngineEnum};
use godot::prelude::*;
use godot::register::info::{PropertyHint, PropertyUsageFlags};
use godot::sys;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use crate::ancestry::{Ancestry, Lineage};
use crate::bridge::{self, NameKey, Owner, ToEngine, ToRuby};
use crate::error;
use crate::header::Header;
use crate::log::GodotLog;
use crate::realm::{self, Build, Key, RubyError};
use crate::snapshot::{self, Heading, Member, Property, Snapshot};

/// A node's instance of a `RubyScript`. It holds no Ruby value: the node's
/// Ruby object is built in the game's realm the first time Godot calls a
/// method its class defines, unless Ruby made the node, and the realm holds
/// it under the node's key. Nothing in it changes but its stage, so Godot
/// may ask it from any thread.
pub struct RubyInstance {
    script: Gd<Script>,
    // Shared with the calls running Ruby, which outlive the instance when
    // Ruby takes its node's script away.
    caller: Arc<Caller>,
    // What its header and ancestry define, shared with every instance made
    // from them.
    methods: Arc<Methods>,
    // The engine class of the node itself, which is the script's engine
    // class or one descending from it, and whose properties are the
    // engine's to answer.
    class_name: StringName,
    // The language its script reports, which Godot also asks the instance for.
    language: Gd<ScriptLanguage>,
    // What the node prints as while its script says nothing about it.
    display: GString,
}

/// The methods a node's class has, by the StringName Godot calls each with,
/// for every instance of one header and ancestry: worked out again whenever
/// another snapshot is published, so a call finds its method's name without
/// formatting it or walking the lineage.
#[derive(Default)]
pub struct Methods(Mutex<Option<MethodTable>>);

// The methods a lineage had in a snapshot, and each one's name.
struct MethodTable {
    snapshot: Arc<Snapshot>,
    names: FxHashMap<NameKey, Arc<str>>,
}

impl Methods {
    // The name of `method` when the class has it, read from the latest
    // snapshot and the lineage `lineage` gives.
    fn name_by_key<'l>(
        &self,
        method: &StringName,
        lineage: impl FnOnce() -> Lineage<'l>,
    ) -> Option<Arc<str>> {
        let latest = snapshot::latest();
        let mut table = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let table = match &mut *table {
            Some(table) if Arc::ptr_eq(&table.snapshot, &latest) => table,
            stale => stale.insert(MethodTable::new(&lineage(), latest)),
        };
        table.names.get(&bridge::read_identity(method)).cloned()
    }
}

impl MethodTable {
    fn new(lineage: &Lineage, snapshot: Arc<Snapshot>) -> Self {
        let names = lineage
            .collect_methods(&snapshot)
            .into_iter()
            .map(|name| (NameKey::new(StringName::from(name)), Arc::from(name)))
            .collect();
        Self { snapshot, names }
    }
}

/// What Godot wrote to a node's properties before it had a Ruby object, in
/// the order it wrote them: a node made for a scene is given the scene's
/// values before anything builds it.
#[derive(Default)]
struct Stash(Vec<(String, Variant)>);

// SAFETY: the mutex holding it lets one thread reach it at a time, and the
// engine's values are shared across threads under gdext's
// experimental-threads.
unsafe impl Send for Stash {}

impl Stash {
    // What was written to the property `name`, if anything was.
    fn value(&self, name: &str) -> Option<Variant> {
        self.0
            .iter()
            .find(|(written, _)| written == name)
            .map(|(_, value)| value.clone())
    }

    // Keeps `value` for the property `name`, in place of what was written to
    // it before.
    fn keep(&mut self, name: &str, value: &Variant) {
        self.0.retain(|(written, _)| written != name);
        self.0.push((name.to_owned(), value.clone()));
    }
}

/// How far a node's Ruby object has come.
#[derive(Clone, Copy)]
enum Stage {
    /// Not built yet: making a node runs no Ruby.
    Recorded,
    Built(Key),
    /// Its file or `initialize` raised, which was reported; the node calls
    /// nothing from then on, as an engine-made object is never built again.
    Failed,
}

// Each Stage as a Caller holds it.
const RECORDED: u8 = 0;
const BUILT: u8 = 1;
const FAILED: u8 = 2;

impl RubyInstance {
    pub fn new(
        script: Gd<Script>,
        header: Arc<Header>,
        ancestry: Arc<Ancestry>,
        methods: Arc<Methods>,
        language: Gd<ScriptLanguage>,
        owner: &Gd<Object>,
    ) -> Self {
        Self {
            caller: Arc::new(Caller {
                path: script.get_path().to_string(),
                owner: owner.instance_id(),
                header,
                ancestry,
                stage: AtomicU8::new(RECORDED),
                stash: Mutex::new(Stash::default()),
            }),
            script,
            methods,
            class_name: StringName::from(&owner.get_class()),
            language,
            display: GString::from(&owner.to_string()),
        }
    }

    /// Hands the instance to Godot for the node it was made for, which frees
    /// it with the node or when the node's script is set.
    pub fn into_godot(self) -> RawPtr<*mut c_void> {
        let data = Box::into_raw(Box::new(self));
        // SAFETY: the interface is initialized while the extension runs;
        // `INFO` is static, and Godot hands `data` back to each callback
        // until `free` takes it.
        unsafe {
            let created = sys::interface_fn!(script_instance_create3)(&INFO, data.cast());
            RawPtr::new(created.cast::<c_void>())
        }
    }

    // Whether the node's class defines `method` or inherits it from a file,
    // whether its source writes it or its class defined it as it ran.
    fn has_method(&self, method: &str) -> bool {
        self.lineage().has_method(&snapshot::latest(), method)
    }

    // The name of `method`, which Godot calls by its StringName, when the
    // node's class has it.
    fn method_name_by_key(&self, method: &StringName) -> Option<Arc<str>> {
        self.methods.name_by_key(method, || self.lineage())
    }

    fn lineage(&self) -> Lineage<'_> {
        self.caller.lineage()
    }

    // What the node's class declared for the editor, its ancestors' included
    // and in the order each class wrote it; a file that has not run has the
    // properties its header writes and no heading.
    fn members(&self) -> Vec<Member> {
        self.lineage().collect_members(&snapshot::latest())
    }

    // The methods the node's class has, its ancestors' included: the ones
    // their sources define and the ones their classes defined as they ran.
    fn methods(&self) -> BTreeSet<String> {
        let snapshot = snapshot::latest();
        self.lineage()
            .files()
            .flat_map(|(path, header)| {
                header
                    .methods()
                    .map(str::to_owned)
                    .chain(snapshot.methods_by_path(path).iter().cloned())
            })
            .collect()
    }

    // Whether the node has no Ruby object yet, so what Godot writes has
    // nowhere to go but the instance.
    fn is_unbuilt(&self) -> bool {
        matches!(self.caller.current_stage(), Stage::Recorded)
    }

    // Whether the node's class exported a property of that name.
    fn has_export(&self, name: &str) -> bool {
        self.lineage()
            .property_by_name(&snapshot::latest(), name)
            .is_some()
    }

    // Whether the node's own engine class has a property of that name, which
    // is the engine's to answer even where the Ruby object holds one too. A
    // node may descend from the class its script extends, so the class asked
    // is the node's rather than the script's.
    fn has_engine_property(&self, name: &str) -> bool {
        let mut class_db = ClassDb::singleton();
        !class_db
            .class_get_property_setter(&self.class_name, name)
            .is_empty()
            || !class_db
                .class_get_property_getter(&self.class_name, name)
                .is_empty()
    }

    // What a call into Ruby needs of the instance, taken before Ruby runs.
    fn caller(&self) -> Arc<Caller> {
        Arc::clone(&self.caller)
    }
}

// Godot frees an instance with its node, on whatever thread frees the node,
// or when the node's script is set, even while a call into it runs.
impl Drop for RubyInstance {
    fn drop(&mut self) {
        if let Stage::Built(key) = self.caller.current_stage() {
            realm::release(key);
        }
    }
}

/// What calls a method on a node's Ruby object: the part of the instance a
/// call into Ruby shares, so the engine may call back into the instance or
/// free it while Ruby runs.
struct Caller {
    path: String,
    owner: InstanceId,
    // The header and ancestry its script had as the instance was made, which
    // answer which methods the node's class has without entering the realm.
    header: Arc<Header>,
    ancestry: Arc<Ancestry>,
    // The Stage the node's object has reached, read on every call without a
    // lock; a built object is always held under the node's own key.
    stage: AtomicU8,
    // What Godot wrote to the node's properties before it had a Ruby object,
    // waiting for the object to be built.
    stash: Mutex<Stash>,
}

impl Caller {
    fn current_stage(&self) -> Stage {
        match self.stage.load(Ordering::Acquire) {
            BUILT => Stage::Built(bridge::node_key(self.owner)),
            FAILED => Stage::Failed,
            _ => Stage::Recorded,
        }
    }

    fn settle(&self, stage: Stage) {
        let stage = match stage {
            Stage::Recorded => RECORDED,
            Stage::Built(key) => {
                debug_assert_eq!(key, bridge::node_key(self.owner));
                BUILT
            }
            Stage::Failed => FAILED,
        };
        self.stage.store(stage, Ordering::Release);
    }

    // Whether building the node's object failed, so it never has one.
    fn is_failed(&self) -> bool {
        matches!(self.current_stage(), Stage::Failed)
    }

    fn lineage(&self) -> Lineage<'_> {
        Lineage::new(
            self.path.clone(),
            &self.header,
            Some(Arc::clone(&self.ancestry)),
        )
    }

    // Keeps `value` for the property `name` while the node has no object:
    // until it is built, when a class's own values are written too, or for
    // good once building it failed, so a scene saved with it keeps the value.
    fn keep(&self, name: &str, value: &Variant) {
        self.stash
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .keep(name, value);
    }

    // What answers the property `name` while the node has no object: what
    // Godot wrote to it, or else the value its class exported it with.
    fn answer_from_stash(&self, name: &str) -> Option<Variant> {
        let written = self
            .stash
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .value(name);
        written.or_else(|| {
            self.lineage()
                .property_by_name(&snapshot::latest(), name)
                .map(|property| property.default_value())
        })
    }

    // The node's Ruby object, built at the first call that needs it and
    // initialized once it is held, unless Ruby made the node and built it.
    // A call arriving while it initializes finds it held; one arriving while
    // its file runs, before the class exists, finds none and builds nothing.
    fn object(&self) -> Option<Key> {
        match self.current_stage() {
            Stage::Built(key) => return Some(key),
            Stage::Failed => return None,
            Stage::Recorded => {}
        }
        let key = bridge::node_key(self.owner);
        let owner = [Owner(self.owner)];
        let built = realm::enter(|realm| realm.build(&self.path, key, c"__build__", owner))
            .and_then(|built| {
                if built == Build::Pending {
                    return Ok(None);
                }
                self.settle(Stage::Built(key));
                if built == Build::New {
                    realm::enter(|realm| realm.send::<ToRuby, ToEngine>(key, "initialize", []))?;
                }
                self.write_stash(key);
                Ok(Some(key))
            });
        built
            .inspect_err(|failed| {
                failed.write(&GodotLog);
                realm::release(key);
                self.settle(Stage::Failed);
            })
            .ok()
            .flatten()
    }

    // Writes what Godot wrote to the node's properties before it had an
    // object, in the order it wrote them and after `initialize`, so a class
    // sets itself up before the scene it was made for has its say.
    fn write_stash(&self, key: Key) {
        let staged =
            std::mem::take(&mut self.stash.lock().unwrap_or_else(PoisonError::into_inner).0);
        if staged.is_empty() {
            return;
        }
        let properties = self.lineage().collect_properties(&snapshot::latest());
        for (name, value) in staged {
            let exported = properties.iter().any(|property| property.name == name);
            write(key, &name, exported, &value);
        }
    }

    // Calls `method` on the node's Ruby object; an exception, or an argument
    // that cannot reach Ruby, is reported and answers null, as a callback
    // that returned nothing does.
    fn send(&self, method: &str, args: &[&Variant]) -> Variant {
        let Some(key) = self.object() else {
            return Variant::nil();
        };
        let checked = args.iter().map(|arg| ToRuby::try_new(arg));
        let args = match checked.collect::<Result<SmallVec<[_; 4]>, _>>() {
            Ok(args) => args,
            Err(reason) => {
                error!("#{method} was not called: {reason}");
                return Variant::nil();
            }
        };
        realm::enter(|realm| realm.send::<_, ToEngine>(key, method, args))
            .map(|ToEngine(answer)| answer)
            .unwrap_or_else(|failed: RubyError| {
                failed.write(&GodotLog);
                Variant::nil()
            })
    }
}

// Writes `value` to the property `name` of the object `key` holds: through
// the property's setter when its class exported it, and otherwise into an
// instance variable the object wrote itself. Whether it was written is what
// Godot takes for an answer, so a refusal leaves the name to the engine.
fn write(key: Key, name: &str, exported: bool, value: &Variant) -> bool {
    let Ok(value) = ToRuby::try_new(value) else {
        return false;
    };
    let written = if exported {
        let setter = format!("{name}=");
        realm::enter(|realm| realm.send::<_, ToEngine>(key, &setter, [value])).map(|_| true)
    } else {
        let variable = StringName::from(name).to_variant();
        let Ok(variable) = ToRuby::try_new(&variable) else {
            return false;
        };
        realm::enter(|realm| realm.send::<_, bool>(key, "__write_variable__", [variable, value]))
    };
    written.unwrap_or_else(|failed: RubyError| {
        failed.write(&GodotLog);
        false
    })
}

// What the property `name` of the object `key` holds: what the property's
// getter answers when its class exported it, and otherwise an instance
// variable the object wrote itself; nothing when it wrote none.
fn read(key: Key, name: &str, exported: bool) -> Option<Variant> {
    let answered = if exported {
        realm::enter(|realm| realm.send::<ToRuby, ToEngine>(key, name, []))
            .map(|ToEngine(answer)| Some(answer))
    } else {
        let variable = StringName::from(name).to_variant();
        let variable = ToRuby::try_new(&variable).ok()?;
        realm::enter(|realm| realm.send::<_, ToEngine>(key, "__read_variable__", [variable]))
            .map(|ToEngine(answer)| (!answer.is_nil()).then(|| answer.to::<VarArray>().at(0)))
    };
    answered.unwrap_or_else(|failed: RubyError| {
        failed.write(&GodotLog);
        None
    })
}

// What a class declared as Godot reads it from an instance, a property or
// the heading the properties after it are shown under.
fn member_info(member: &Member) -> sys::GDExtensionPropertyInfo {
    match member {
        Member::Property(property) => property_info(property),
        Member::Heading(heading) => heading_info(heading),
    }
}

// A heading as Godot reads it from an instance: the name it is headed with,
// the prefix it takes properties by, and the usage saying which kind it is.
fn heading_info(heading: &Heading) -> sys::GDExtensionPropertyInfo {
    sys::GDExtensionPropertyInfo {
        type_: VariantType::NIL.ord() as sys::GDExtensionVariantType,
        name: into_raw(StringName::from(heading.name())),
        class_name: into_raw(StringName::default()),
        hint: PropertyHint::NONE.ord() as u32,
        hint_string: into_raw(GString::from(heading.hint_string())),
        usage: heading.usage().ord() as u32,
    }
}

// An exported property as Godot reads it from an instance: the strings it
// points at are the array's own, taken back when Godot hands the array to
// `free_property_list`.
fn property_info(property: &Property) -> sys::GDExtensionPropertyInfo {
    let usage = PropertyUsageFlags::DEFAULT.ord() | PropertyUsageFlags::SCRIPT_VARIABLE.ord();
    sys::GDExtensionPropertyInfo {
        type_: property.kind.ord() as sys::GDExtensionVariantType,
        name: into_raw(StringName::from(&property.name)),
        class_name: into_raw(StringName::default()),
        hint: property.hint.ord() as u32,
        hint_string: into_raw(GString::from(&property.hint_string)),
        usage: usage as u32,
    }
}

// A method as Godot reads it from an instance: its name, which is all a Ruby
// class says about it, and an answer of any type, as an untyped GDScript
// method's is.
fn method_info(name: &str) -> sys::GDExtensionMethodInfo {
    sys::GDExtensionMethodInfo {
        name: into_raw(StringName::from(name)),
        return_value: answer_info(),
        flags: sys::GDEXTENSION_METHOD_FLAG_NORMAL as u32,
        id: 0,
        argument_count: 0,
        arguments: std::ptr::null_mut(),
        default_argument_count: 0,
        default_arguments: std::ptr::null_mut(),
    }
}

// What a method answers, of any type, since a Ruby method says nothing of
// what it returns.
fn answer_info() -> sys::GDExtensionPropertyInfo {
    sys::GDExtensionPropertyInfo {
        type_: VariantType::NIL.ord() as sys::GDExtensionVariantType,
        name: into_raw(StringName::default()),
        class_name: into_raw(StringName::default()),
        hint: PropertyHint::NONE.ord() as u32,
        hint_string: into_raw(GString::default()),
        usage: PropertyUsageFlags::NIL_IS_VARIANT.ord() as u32,
    }
}

// Whether a name is the engine's rather than the node's class's: a property
// its engine class has is the engine's, whatever the Ruby object holds under
// that name, and so is a name Godot spells with a slash, as metadata and
// property groups are.
fn is_engines_own(instance: &RubyInstance, name: &str) -> bool {
    instance.has_engine_property(name) || name.contains('/')
}

// A string the array holds for Godot to read until it hands the array back,
// as the pointer the engine's property info keeps it under.
fn into_raw<T, P>(string: T) -> *mut P {
    Box::into_raw(Box::new(string)).cast::<P>()
}

// SAFETY: `string` is what `into_raw` made for this array, taken back once.
unsafe fn drop_raw<T, P>(string: *mut P) {
    drop(unsafe { Box::from_raw(string.cast::<T>()) });
}

// What Godot calls on a Ruby instance. A callback not given leaves Godot's
// default: no fallback, and the property state Godot gathers from the
// property list and `get`.
static INFO: sys::GDExtensionScriptInstanceInfo3 = sys::GDExtensionScriptInstanceInfo3 {
    set_func: Some(set),
    get_func: Some(get),
    get_property_list_func: Some(get_property_list),
    free_property_list_func: Some(free_property_list),
    get_class_category_func: Some(get_class_category),
    property_can_revert_func: Some(property_can_revert),
    property_get_revert_func: Some(property_get_revert),
    get_owner_func: None,
    get_property_state_func: None,
    get_method_list_func: Some(get_method_list),
    free_method_list_func: Some(free_method_list),
    get_property_type_func: None,
    validate_property_func: None,
    has_method_func: Some(has_method),
    get_method_argument_count_func: None,
    call_func: Some(call),
    notification_func: Some(notification),
    to_string_func: Some(to_string),
    refcount_incremented_func: None,
    refcount_decremented_func: Some(refcount_decremented),
    get_script_func: Some(get_script),
    is_placeholder_func: None,
    set_fallback_func: None,
    get_fallback_func: None,
    get_language_func: Some(get_language),
    free_func: Some(free),
};

// The instance Godot hands a callback, shared by callbacks on several
// threads, since nothing changes it but its stage, behind a mutex.
//
// SAFETY: `data` is what `into_godot` handed Godot, alive until `free`; the
// reference must not be used once Ruby runs, since Ruby may have Godot free
// the instance.
unsafe fn instance<'a>(data: sys::GDExtensionScriptInstanceDataPtr) -> &'a RubyInstance {
    unsafe { &*data.cast::<RubyInstance>() }
}

// SAFETY: `method` is a StringName Godot hands for the call, which
// `StringName` lays out as.
unsafe fn name(method: sys::GDExtensionConstStringNamePtr) -> String {
    unsafe { &*method.cast::<StringName>() }.to_string()
}

// A property of the node's class is its Ruby object's to answer, so a
// thread inside the realm has the object built as a call builds it, and one
// outside leaves the value with the instance rather than waiting for the
// realm: the object is given it once something builds it. A node whose
// object failed to build keeps what is written to it, as a GDScript tool
// keeps its members after an error, so saving its scene loses nothing.
unsafe extern "C" fn set(
    data: sys::GDExtensionScriptInstanceDataPtr,
    property: sys::GDExtensionConstStringNamePtr,
    value: sys::GDExtensionConstVariantPtr,
) -> sys::GDExtensionBool {
    // SAFETY: the property name lives for the call.
    let name = unsafe { name(property) };
    // SAFETY: Godot hands a live variant for the call.
    let value = unsafe { &*value.cast::<Variant>() };
    // SAFETY: the instance lives until Ruby runs, and is not used after.
    let reached = {
        let instance = unsafe { instance(data) };
        let exported = instance.has_export(&name);
        if !exported && is_engines_own(instance, &name) {
            return sys::GDExtensionBool::from(false);
        }
        let is_deferred = instance.is_unbuilt() && !realm::is_inside();
        (instance.caller(), exported, is_deferred)
    };
    let (caller, exported, is_deferred) = reached;
    let key = if is_deferred { None } else { caller.object() };
    let is_written = match key {
        Some(key) => write(key, &name, exported, value),
        None if is_deferred || caller.is_failed() => {
            caller.keep(&name, value);
            true
        }
        None => false,
    };
    sys::GDExtensionBool::from(is_written)
}

unsafe extern "C" fn get(
    data: sys::GDExtensionScriptInstanceDataPtr,
    property: sys::GDExtensionConstStringNamePtr,
    answer: sys::GDExtensionVariantPtr,
) -> sys::GDExtensionBool {
    // SAFETY: the property name lives for the call.
    let name = unsafe { name(property) };
    // SAFETY: the instance lives until Ruby runs, and is not used after.
    let reached = {
        let instance = unsafe { instance(data) };
        let exported = instance.has_export(&name);
        if !exported && is_engines_own(instance, &name) {
            return sys::GDExtensionBool::from(false);
        }
        let is_deferred = instance.is_unbuilt() && !realm::is_inside();
        (instance.caller(), exported, is_deferred)
    };
    let (caller, exported, is_deferred) = reached;
    let key = if is_deferred { None } else { caller.object() };
    let value = match key {
        Some(key) => read(key, &name, exported),
        None if is_deferred || caller.is_failed() => caller.answer_from_stash(&name),
        None => None,
    };
    let Some(value) = value else {
        return sys::GDExtensionBool::from(false);
    };
    // SAFETY: Godot hands a variant of its own to write the answer into.
    unsafe { *answer.cast::<Variant>() = value };
    sys::GDExtensionBool::from(true)
}

// The properties of the node's class, as the array Godot reads and hands
// back to `free_property_list`. Its class has them once its file has run, so
// a node whose file has not run yet has none.
// Godot heads an instance's properties with its script's own category when
// the instance names none; the list carries one for every class in the
// chain, so it is answered with none of its own.
unsafe extern "C" fn get_class_category(
    _data: sys::GDExtensionScriptInstanceDataPtr,
    _category: *mut sys::GDExtensionPropertyInfo,
) -> sys::GDExtensionBool {
    sys::GDExtensionBool::from(false)
}

unsafe extern "C" fn get_property_list(
    data: sys::GDExtensionScriptInstanceDataPtr,
    count: *mut u32,
) -> *const sys::GDExtensionPropertyInfo {
    // SAFETY: the instance lives for this call, which runs no Ruby.
    let members = unsafe { instance(data) }.members();
    let infos: Box<[sys::GDExtensionPropertyInfo]> = members.iter().map(member_info).collect();
    // SAFETY: Godot hands a count to fill.
    unsafe { *count = infos.len() as u32 };
    Box::into_raw(infos).cast::<sys::GDExtensionPropertyInfo>()
}

unsafe extern "C" fn free_property_list(
    _data: sys::GDExtensionScriptInstanceDataPtr,
    list: *const sys::GDExtensionPropertyInfo,
    count: u32,
) {
    // SAFETY: Godot hands back what `get_property_list` made, once, with the
    // count it was given.
    let infos = unsafe {
        Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            list.cast_mut(),
            count as usize,
        ))
    };
    for info in &infos {
        // SAFETY: each string is what `property_info` made for this array.
        unsafe {
            drop_raw::<StringName, _>(info.name);
            drop_raw::<StringName, _>(info.class_name);
            drop_raw::<GString, _>(info.hint_string);
        }
    }
}

unsafe extern "C" fn get_method_list(
    data: sys::GDExtensionScriptInstanceDataPtr,
    count: *mut u32,
) -> *const sys::GDExtensionMethodInfo {
    // SAFETY: the instance lives for this call, which runs no Ruby.
    let methods = unsafe { instance(data) }.methods();
    let infos: Box<[sys::GDExtensionMethodInfo]> = methods
        .iter()
        .map(|method| method_info(method.as_str()))
        .collect();
    // SAFETY: Godot hands a count to fill.
    unsafe { *count = infos.len() as u32 };
    Box::into_raw(infos).cast::<sys::GDExtensionMethodInfo>()
}

unsafe extern "C" fn free_method_list(
    _data: sys::GDExtensionScriptInstanceDataPtr,
    list: *const sys::GDExtensionMethodInfo,
    count: u32,
) {
    // SAFETY: Godot hands back what `get_method_list` made, once, with the
    // count it was given.
    let infos = unsafe {
        Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            list.cast_mut(),
            count as usize,
        ))
    };
    for info in &infos {
        // SAFETY: each string is what `method_info` made for this array.
        unsafe {
            drop_raw::<StringName, _>(info.name);
            drop_raw::<StringName, _>(info.return_value.name);
            drop_raw::<StringName, _>(info.return_value.class_name);
            drop_raw::<GString, _>(info.return_value.hint_string);
        }
    }
}

unsafe extern "C" fn has_method(
    data: sys::GDExtensionScriptInstanceDataPtr,
    method: sys::GDExtensionConstStringNamePtr,
) -> sys::GDExtensionBool {
    // SAFETY: the instance and the method name live for this call, which
    // runs no Ruby.
    let has = unsafe { instance(data).method_name_by_key(&*method.cast::<StringName>()) }.is_some();
    sys::GDExtensionBool::from(has)
}

// Whether the property can be reverted, as the class's own
// `_property_can_revert` answers with true, as a GDScript's does; the
// default an exported property reverts to is the script's to answer.
unsafe extern "C" fn property_can_revert(
    data: sys::GDExtensionScriptInstanceDataPtr,
    property: sys::GDExtensionConstStringNamePtr,
) -> sys::GDExtensionBool {
    // SAFETY: the property name lives for the call, and the instance until
    // Ruby runs.
    let answer = unsafe { revert_answer(data, property, "_property_can_revert") };
    sys::GDExtensionBool::from(answer.try_to::<bool>().unwrap_or(false))
}

// The value the property reverts to, as the class's own
// `_property_get_revert` answers with anything but nil, as a GDScript's does.
unsafe extern "C" fn property_get_revert(
    data: sys::GDExtensionScriptInstanceDataPtr,
    property: sys::GDExtensionConstStringNamePtr,
    reverted: sys::GDExtensionVariantPtr,
) -> sys::GDExtensionBool {
    // SAFETY: the property name lives for the call, and the instance until
    // Ruby runs.
    let answer = unsafe { revert_answer(data, property, "_property_get_revert") };
    if answer.is_nil() {
        return sys::GDExtensionBool::from(false);
    }
    // SAFETY: Godot hands an initialized variant of its own to fill.
    unsafe { *reverted.cast::<Variant>() = answer };
    sys::GDExtensionBool::from(true)
}

// What the class's `method` answers for the property, or nil when the class
// does not define it.
//
// SAFETY: `property` lives for the call, and the instance until Ruby runs.
unsafe fn revert_answer(
    data: sys::GDExtensionScriptInstanceDataPtr,
    property: sys::GDExtensionConstStringNamePtr,
    method: &str,
) -> Variant {
    // SAFETY: as the caller promises.
    let property = StringName::from(&unsafe { name(property) });
    let caller = {
        // SAFETY: as the caller promises.
        let instance = unsafe { instance(data) };
        if !instance.has_method(method) {
            return Variant::nil();
        }
        instance.caller()
    };
    caller.send(method, &[&property.to_variant()])
}

unsafe extern "C" fn call(
    data: sys::GDExtensionScriptInstanceDataPtr,
    method: sys::GDExtensionConstStringNamePtr,
    args: *const sys::GDExtensionConstVariantPtr,
    count: sys::GDExtensionInt,
    answer: sys::GDExtensionVariantPtr,
    error: *mut sys::GDExtensionCallError,
) {
    // SAFETY: the instance lives until Ruby runs, and is not used after;
    // the method name lives for the call.
    let (method, caller) = {
        let instance = unsafe { instance(data) };
        let Some(method) = instance.method_name_by_key(unsafe { &*method.cast::<StringName>() })
        else {
            // SAFETY: Godot hands an error to fill.
            unsafe { (*error).error = sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD };
            return;
        };
        (method, instance.caller())
    };
    let args: &[&Variant] = if args.is_null() {
        &[]
    } else {
        // SAFETY: Godot hands `count` live variants, which `Variant` lays out
        // as, alive for the call.
        unsafe { std::slice::from_raw_parts(args.cast::<&Variant>(), count as usize) }
    };
    let answered = caller.send(&method, args);
    // SAFETY: Godot hands an initialized variant and an error to fill, both
    // its own, so they outlive the instance.
    unsafe {
        *answer.cast::<Variant>() = answered;
        (*error).error = sys::GDEXTENSION_CALL_OK;
    }
}

// The name Godot forwards every notification to a script by.
static NOTIFICATION: LazyLock<StringName> = LazyLock::new(|| StringName::from("_notification"));

unsafe extern "C" fn notification(
    data: sys::GDExtensionScriptInstanceDataPtr,
    what: i32,
    _reversed: sys::GDExtensionBool,
) {
    // SAFETY: the instance lives until Ruby runs, and is not used after.
    let caller = {
        let instance = unsafe { instance(data) };
        if instance.method_name_by_key(&NOTIFICATION).is_none() {
            return;
        }
        instance.caller()
    };
    caller.send("_notification", &[&what.to_variant()]);
}

unsafe extern "C" fn to_string(
    data: sys::GDExtensionScriptInstanceDataPtr,
    valid: *mut sys::GDExtensionBool,
    out: sys::GDExtensionStringPtr,
) {
    // SAFETY: the instance lives for this call, which runs no Ruby; Godot
    // hands an initialized String, which `GString` lays out as, and a flag.
    unsafe {
        *out.cast::<GString>() = instance(data).display.clone();
        *valid = sys::GDExtensionBool::from(true);
    }
}

// The node may be freed once nothing else refers to it: its Ruby object
// holds it through its key, not a reference.
unsafe extern "C" fn refcount_decremented(
    _data: sys::GDExtensionScriptInstanceDataPtr,
) -> sys::GDExtensionBool {
    sys::GDExtensionBool::from(true)
}

unsafe extern "C" fn get_script(
    data: sys::GDExtensionScriptInstanceDataPtr,
) -> sys::GDExtensionObjectPtr {
    // SAFETY: the instance lives for this call; Godot takes its own reference.
    unsafe { instance(data).script.obj_sys() }
}

unsafe extern "C" fn get_language(
    data: sys::GDExtensionScriptInstanceDataPtr,
) -> sys::GDExtensionScriptLanguagePtr {
    // SAFETY: the instance lives for this call.
    unsafe { instance(data).language.obj_sys().cast() }
}

unsafe extern "C" fn free(data: sys::GDExtensionScriptInstanceDataPtr) {
    // SAFETY: Godot frees an instance once, handing back what `into_godot`
    // boxed; a call still running holds only its `Caller`.
    drop(unsafe { Box::from_raw(data.cast::<RubyInstance>()) });
}
