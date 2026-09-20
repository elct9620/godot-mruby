//! A node's instance of a `RubyScript`, handed to Godot through the
//! extension interface directly rather than gdext's `ScriptInstance`: Godot
//! deletes an instance the moment its node's script is set, even while a
//! call into it is running, and gdext keeps the instance borrowed for the
//! whole call. Each callback here borrows the instance only until Ruby is
//! about to run, and touches nothing of it afterwards, as GDScript, C# and
//! other languages' instances do.

use std::ffi::c_void;
use std::sync::{Arc, Mutex, PoisonError};

use godot::classes::{ClassDb, Object, Script, ScriptLanguage};
use godot::meta::conv::RawPtr;
use godot::obj::{EngineBitfield, EngineEnum};
use godot::prelude::*;
use godot::register::info::{PropertyHint, PropertyUsageFlags};
use godot::sys;

use crate::ancestry::Ancestry;
use crate::bridge::{self, Owner, ToEngine, ToRuby};
use crate::error;
use crate::log::GodotLog;
use crate::parser::Header;
use crate::realm::{self, Built, Key, RubyError};
use crate::snapshot::{self, Group, Member, Property};

/// A node's instance of a `RubyScript`. It holds no Ruby value: the node's
/// Ruby object is built in the game's realm the first time Godot calls a
/// method its class defines, unless Ruby made the node, and the realm holds
/// it under the node's key. Nothing in it changes but its stage, so Godot
/// may ask it from any thread.
pub struct RubyInstance {
    script: Gd<Script>,
    path: String,
    owner: InstanceId,
    // The header and ancestry its script had as the instance was made, which
    // answer which methods the node's class has without entering the realm.
    header: Arc<Header>,
    ancestry: Arc<Ancestry>,
    // The engine class of the node itself, which is the script's engine
    // class or one descending from it, and whose properties are the
    // engine's to answer.
    class_name: StringName,
    // The language its script reports, which Godot also asks the instance for.
    language: Gd<ScriptLanguage>,
    // What the node prints as while its script says nothing about it.
    display: GString,
    // Shared with the calls running Ruby, which outlive the instance when
    // Ruby takes its node's script away.
    stage: Arc<Mutex<Stage>>,
    // What Godot wrote to the node's properties before it had a Ruby object,
    // waiting for the object to be built.
    staged: Arc<Mutex<Staged>>,
}

/// What Godot wrote to a node's properties before it had a Ruby object, in
/// the order it wrote them: a node made for a scene is given the scene's
/// values before anything builds it.
#[derive(Default)]
struct Staged(Vec<(String, Variant)>);

// SAFETY: the mutex holding it lets one thread reach it at a time, and the
// engine's values are shared across threads under gdext's
// experimental-threads.
unsafe impl Send for Staged {}

impl Staged {
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

impl RubyInstance {
    pub fn new(
        script: Gd<Script>,
        header: Arc<Header>,
        ancestry: Arc<Ancestry>,
        language: Gd<ScriptLanguage>,
        owner: &Gd<Object>,
    ) -> Self {
        Self {
            path: script.get_path().to_string(),
            script,
            owner: owner.instance_id(),
            header,
            ancestry,
            class_name: StringName::from(&owner.get_class()),
            language,
            display: GString::from(&owner.to_string()),
            stage: Arc::new(Mutex::new(Stage::Recorded)),
            staged: Arc::new(Mutex::new(Staged::default())),
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
    fn has(&self, method: &str) -> bool {
        let snapshot = snapshot::latest();
        self.header.has_method(method)
            || self.ancestry.has_method(method)
            || snapshot.has_method(&self.path, method)
            || self
                .ancestry
                .paths()
                .any(|path| snapshot.has_method(path, method))
    }

    // What the node's class declared for the editor, its ancestors' included
    // and in the order each class wrote it; none until its file has run.
    fn members(&self) -> Vec<Member> {
        snapshot::latest().listing_of(declaring_paths(&self.path, &self.ancestry))
    }

    // The files the node's class takes its shape from: its own, then the
    // ones it inherits from, nearest first.
    fn declaring_paths(&self) -> impl Iterator<Item = &str> {
        declaring_paths(&self.path, &self.ancestry)
    }

    // What Godot wrote to the property `name` before the node had a Ruby
    // object, if it wrote one.
    fn staged(&self, name: &str) -> Option<Variant> {
        self.staged
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .value(name)
    }

    // The value the class exported the property with, which answers Godot
    // while the node has no object of its own to answer from.
    fn default_value(&self, name: &str) -> Option<Variant> {
        let snapshot = snapshot::latest();
        self.declaring_paths()
            .find_map(|path| {
                snapshot
                    .properties(path)
                    .find(|property| property.name == name)
            })
            .map(Property::default_value)
    }

    // Whether the node has no Ruby object yet, so what Godot writes has
    // nowhere to go but the instance.
    fn unbuilt(&self) -> bool {
        matches!(
            *self.stage.lock().unwrap_or_else(PoisonError::into_inner),
            Stage::Recorded
        )
    }

    // Keeps `value` for the property `name` until the node's Ruby object is
    // built, which is when a class's own values are written too.
    fn stage(&self, name: &str, value: &Variant) {
        self.staged
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .keep(name, value);
    }

    // Whether the node's class exported a property of that name.
    fn exports(&self, name: &str) -> bool {
        let snapshot = snapshot::latest();
        self.declaring_paths().any(|path| {
            snapshot
                .properties(path)
                .any(|property| property.name == name)
        })
    }

    // Whether the node's own engine class has a property of that name, which
    // is the engine's to answer even where the Ruby object holds one too. A
    // node may descend from the class its script extends, so the class asked
    // is the node's rather than the script's.
    fn engine_property(&self, name: &str) -> bool {
        let mut class_db = ClassDb::singleton();
        !class_db
            .class_get_property_setter(&self.class_name, name)
            .is_empty()
            || !class_db
                .class_get_property_getter(&self.class_name, name)
                .is_empty()
    }

    // What a call into Ruby needs of the instance, taken before Ruby runs.
    fn caller(&self) -> Caller {
        Caller {
            path: self.path.clone(),
            owner: self.owner,
            ancestry: Arc::clone(&self.ancestry),
            stage: Arc::clone(&self.stage),
            staged: Arc::clone(&self.staged),
        }
    }
}

// Godot frees an instance with its node, on whatever thread frees the node,
// or when the node's script is set, even while a call into it runs.
impl Drop for RubyInstance {
    fn drop(&mut self) {
        if let Stage::Built(key) = *self.stage.lock().unwrap_or_else(PoisonError::into_inner) {
            realm::release(key);
        }
    }
}

/// What calls a method on a node's Ruby object, holding nothing of the
/// instance itself, so the engine may call back into the instance or free it
/// while Ruby runs.
struct Caller {
    path: String,
    owner: InstanceId,
    ancestry: Arc<Ancestry>,
    stage: Arc<Mutex<Stage>>,
    staged: Arc<Mutex<Staged>>,
}

impl Caller {
    fn stage(&self) -> Stage {
        *self.stage.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn settle(&self, stage: Stage) {
        *self.stage.lock().unwrap_or_else(PoisonError::into_inner) = stage;
    }

    // The node's Ruby object, built at the first call that needs it and
    // initialized once it is held, unless Ruby made the node and built it.
    // A call arriving while it initializes finds it held; one arriving while
    // its file runs, before the class exists, finds none and builds nothing.
    fn object(&self) -> Option<Key> {
        match self.stage() {
            Stage::Built(key) => return Some(key),
            Stage::Failed => return None,
            Stage::Recorded => {}
        }
        let key = bridge::node_key(self.owner);
        let owner = [Owner(self.owner)];
        let built = realm::enter(|realm| realm.build(&self.path, key, c"__build__", owner))
            .and_then(|built| {
                if built == Built::Waiting {
                    return Ok(None);
                }
                self.settle(Stage::Built(key));
                if built == Built::Made {
                    realm::enter(|realm| realm.send::<ToRuby, ToEngine>(key, "initialize", []))?;
                }
                self.write_staged(key);
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
    fn write_staged(&self, key: Key) {
        let staged =
            std::mem::take(&mut self.staged.lock().unwrap_or_else(PoisonError::into_inner).0);
        if staged.is_empty() {
            return;
        }
        let properties = properties_of(&self.path, &self.ancestry);
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
        let checked = args.iter().map(|arg| ToRuby::checked(arg));
        let args = match checked.collect::<Result<Vec<_>, _>>() {
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

// The properties the class of the file at `path` exported, its ancestors'
// included, nearest first.
fn properties_of(path: &str, ancestry: &Ancestry) -> Vec<Property> {
    snapshot::latest().properties_of(declaring_paths(path, ancestry))
}

// The files the class of the file at `path` takes its shape from: its own,
// then the ones it inherits from, nearest first.
fn declaring_paths<'a>(path: &'a str, ancestry: &'a Ancestry) -> impl Iterator<Item = &'a str> {
    std::iter::once(path).chain(ancestry.paths())
}

// Writes `value` to the property `name` of the object `key` holds: through
// the property's setter when its class exported it, and otherwise into an
// instance variable the object wrote itself. Whether it was written is what
// Godot takes for an answer, so a refusal leaves the name to the engine.
fn write(key: Key, name: &str, exported: bool, value: &Variant) -> bool {
    let Ok(value) = ToRuby::checked(value) else {
        return false;
    };
    let written = if exported {
        let setter = format!("{name}=");
        realm::enter(|realm| realm.send::<_, ToEngine>(key, &setter, [value])).map(|_| true)
    } else {
        let variable = StringName::from(name).to_variant();
        let Ok(variable) = ToRuby::checked(&variable) else {
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
        let variable = ToRuby::checked(&variable).ok()?;
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
        Member::Group(group) => group_info(group),
    }
}

// A heading as Godot reads it from an instance: the name it is headed with,
// the prefix it takes properties by, and the usage saying which kind it is.
fn group_info(group: &Group) -> sys::GDExtensionPropertyInfo {
    sys::GDExtensionPropertyInfo {
        type_: VariantType::NIL.ord() as sys::GDExtensionVariantType,
        name: owned(StringName::from(&group.name)),
        class_name: owned(StringName::default()),
        hint: PropertyHint::NONE.ord() as u32,
        hint_string: owned(GString::from(&group.hint_string)),
        usage: group.usage.ord() as u32,
    }
}

// An exported property as Godot reads it from an instance: the strings it
// points at are the array's own, taken back when Godot hands the array to
// `free_property_list`.
fn property_info(property: &Property) -> sys::GDExtensionPropertyInfo {
    let usage = PropertyUsageFlags::DEFAULT.ord() | PropertyUsageFlags::SCRIPT_VARIABLE.ord();
    sys::GDExtensionPropertyInfo {
        type_: property.kind.ord() as sys::GDExtensionVariantType,
        name: owned(StringName::from(&property.name)),
        class_name: owned(StringName::default()),
        hint: property.hint.ord() as u32,
        hint_string: owned(GString::from(&property.hint_string)),
        usage: usage as u32,
    }
}

// Whether a name is the engine's rather than the node's class's: a property
// its engine class has is the engine's, whatever the Ruby object holds under
// that name, and so is a name Godot spells with a slash, as metadata and
// property groups are.
fn the_engines_own(instance: &RubyInstance, name: &str) -> bool {
    instance.engine_property(name) || name.contains('/')
}

// A string the array holds for Godot to read until it hands the array back,
// as the pointer the engine's property info keeps it under.
fn owned<T, P>(string: T) -> *mut P {
    Box::into_raw(Box::new(string)).cast::<P>()
}

// SAFETY: `string` is what `owned` made for this array, taken back once.
unsafe fn taken<T, P>(string: *mut P) {
    drop(unsafe { Box::from_raw(string.cast::<T>()) });
}

// What Godot calls on a Ruby instance. A callback not given leaves Godot's
// default: no fallback, nothing to revert, and the property state Godot
// gathers from the property list and `get`.
static INFO: sys::GDExtensionScriptInstanceInfo3 = sys::GDExtensionScriptInstanceInfo3 {
    set_func: Some(set),
    get_func: Some(get),
    get_property_list_func: Some(get_property_list),
    free_property_list_func: Some(free_property_list),
    get_class_category_func: Some(get_class_category),
    property_can_revert_func: None,
    property_get_revert_func: None,
    get_owner_func: None,
    get_property_state_func: None,
    get_method_list_func: None,
    free_method_list_func: None,
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
// realm: the object is given it once something builds it.
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
        let exported = instance.exports(&name);
        if !exported && the_engines_own(instance, &name) {
            return sys::GDExtensionBool::from(false);
        }
        if instance.unbuilt() && !realm::inside() {
            instance.stage(&name, value);
            return sys::GDExtensionBool::from(true);
        }
        (instance.caller(), exported)
    };
    let (caller, exported) = reached;
    let Some(key) = caller.object() else {
        return sys::GDExtensionBool::from(false);
    };
    sys::GDExtensionBool::from(write(key, &name, exported, value))
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
        let exported = instance.exports(&name);
        if !exported && the_engines_own(instance, &name) {
            return sys::GDExtensionBool::from(false);
        }
        if instance.unbuilt() && !realm::inside() {
            let staged = instance
                .staged(&name)
                .or_else(|| instance.default_value(&name));
            let Some(answered) = staged else {
                return sys::GDExtensionBool::from(false);
            };
            // SAFETY: Godot hands a variant of its own to write into.
            unsafe { *answer.cast::<Variant>() = answered };
            return sys::GDExtensionBool::from(true);
        }
        (instance.caller(), exported)
    };
    let (caller, exported) = reached;
    let Some(key) = caller.object() else {
        return sys::GDExtensionBool::from(false);
    };
    let Some(answered) = read(key, &name, exported) else {
        return sys::GDExtensionBool::from(false);
    };
    // SAFETY: Godot hands a variant of its own to write the answer into.
    unsafe { *answer.cast::<Variant>() = answered };
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
            taken::<StringName, _>(info.name);
            taken::<StringName, _>(info.class_name);
            taken::<GString, _>(info.hint_string);
        }
    }
}

unsafe extern "C" fn has_method(
    data: sys::GDExtensionScriptInstanceDataPtr,
    method: sys::GDExtensionConstStringNamePtr,
) -> sys::GDExtensionBool {
    // SAFETY: the instance lives for this call, which runs no Ruby.
    let has = unsafe { instance(data).has(&name(method)) };
    sys::GDExtensionBool::from(has)
}

unsafe extern "C" fn call(
    data: sys::GDExtensionScriptInstanceDataPtr,
    method: sys::GDExtensionConstStringNamePtr,
    args: *const sys::GDExtensionConstVariantPtr,
    count: sys::GDExtensionInt,
    answer: sys::GDExtensionVariantPtr,
    error: *mut sys::GDExtensionCallError,
) {
    // SAFETY: the method name lives for the call.
    let method = unsafe { name(method) };
    // SAFETY: the instance lives until Ruby runs, and is not used after.
    let caller = {
        let instance = unsafe { instance(data) };
        if !instance.has(&method) {
            // SAFETY: Godot hands an error to fill.
            unsafe { (*error).error = sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD };
            return;
        }
        instance.caller()
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

unsafe extern "C" fn notification(
    data: sys::GDExtensionScriptInstanceDataPtr,
    what: i32,
    _reversed: sys::GDExtensionBool,
) {
    // SAFETY: the instance lives until Ruby runs, and is not used after.
    let caller = {
        let instance = unsafe { instance(data) };
        if !instance.has("_notification") {
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
