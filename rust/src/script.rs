use std::collections::BTreeSet;
use std::ffi::c_void;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use godot::classes::{
    ClassDb, Engine, IScriptExtension, Object, ResourceLoader, Script, ScriptExtension,
    ScriptLanguage,
};
use godot::global::Error;
use godot::meta::conv::RawPtr;
use godot::obj::{EngineBitfield, EngineEnum};
use godot::prelude::*;
use godot::register::info::{MethodFlags, PropertyUsageFlags};
use rustc_hash::FxHashMap;

use crate::ancestry::{self, Ancestry, Break, Cache, Lineage};
use crate::game::{self, FilesOnDisk, GameFiles};
use crate::header::Header;
use crate::instance::{self, Methods, RubyInstance};
use crate::language;
use crate::realm::{self, Files};
use crate::settings;
use crate::snapshot::{self, Heading, Member, Parameter, Property, Signal};
use crate::{announcement, error};

mod class_instance;
#[cfg(feature = "editor")]
mod editor;

// Placeholders, template marks and help pages exist only in the editor, so a
// library built without it keeps nothing for them and answers with nothing.
#[cfg(not(feature = "editor"))]
mod editor {
    use std::ffi::c_void;

    use godot::meta::conv::RawPtr;
    use godot::prelude::*;

    use super::{RubyScript, no_instance};

    pub fn run_placed(_: bool) {}

    pub fn create_placeholder(_: &RubyScript, _: Gd<Object>) -> RawPtr<*mut c_void> {
        no_instance()
    }

    pub fn erase_placeholder(_: &RubyScript, _: RawPtr<*mut c_void>) {}

    pub fn update_exports(_: &mut RubyScript) {}

    pub fn read_doc_class_name(_: &RubyScript) -> StringName {
        StringName::default()
    }

    pub fn describe_documentation(_: &RubyScript) -> Array<AnyDictionary> {
        Array::new()
    }
}

pub use editor::run_placed;

/// The script a `.rb` file loads as, the way a `.gd` file loads as a `GDScript`.
///
/// It holds the file's source and answers Godot's questions from its header
/// without running any Ruby: a loaded script only becomes code when something
/// enters it.
#[derive(GodotClass)]
#[class(base = ScriptExtension, init, tool)]
pub struct RubyScript {
    base: Base<ScriptExtension>,
    source: GString,
    // Both read again whenever the source changes, so Godot's questions are
    // answered from what the script holds; an instance keeps what its script
    // had as the instance was made.
    header: Arc<Header>,
    // Read from other files' sources at the first question that needs it,
    // since loading a script must not load the scripts it inherits from, and
    // again once any source changes.
    ancestry: Cache,
    // What the header and ancestry its instances were last made from define,
    // shared by every instance made from the same two.
    methods: Mutex<Option<KeptMethods>>,
    #[cfg(feature = "editor")]
    editor_state: editor::EditorState,
    // The nodes whose class a new superclass no longer let the script fit,
    // with what their script variables held, given the script again when a
    // later superclass fits them, as GDScript keeps their reload state.
    refusals: Mutex<Vec<(InstanceId, ScriptVariables)>>,
    // The scripts standing for the classes inside the file's class that Ruby
    // made a node of, by path, kept as a GDScript keeps its inner classes and
    // given the file's source whenever it changes.
    inner_scripts: Mutex<FxHashMap<String, Gd<RubyScript>>>,
}

// What a node's script variables hold, by name.
type ScriptVariables = Vec<(StringName, Variant)>;

// What a header and ancestry define, kept with the two.
type KeptMethods = (Arc<Header>, Arc<Ancestry>, Arc<Methods>);

/// Gives each node of the scripts at `paths`, whose classes were made
/// afresh, a new instance, as GDScript's reload does when a script's base
/// changes: what the node's script variables hold is kept, its script taken
/// away and the objects of the class gone let go, then the script is set
/// again and the values written back. A node the new superclass does not fit
/// is left without the script until a later one does.
pub fn remake_instances(paths: &[String]) {
    let loader = ResourceLoader::singleton();
    let mut owners_by_script = Vec::new();
    for path in paths {
        let Some(script) = loader
            .get_cached_ref(path)
            .and_then(|resource| resource.try_cast::<RubyScript>().ok())
        else {
            continue;
        };
        let mut owners = std::mem::take(&mut *script.bind().refusals());
        for id in instance::owners_by_path(path) {
            let Ok(mut owner) = Gd::<Object>::try_from_instance_id(id) else {
                continue;
            };
            let variables = read_script_variables(&owner);
            owner.set_script(None::<&Gd<Script>>);
            owners.push((id, variables));
        }
        owners_by_script.push((script, owners));
    }
    realm::release_queued();
    for (script, owners) in owners_by_script {
        let base_script = script.clone().upcast::<Script>();
        for (id, variables) in owners {
            let Ok(mut owner) = Gd::<Object>::try_from_instance_id(id) else {
                continue;
            };
            owner.set_script(&base_script);
            if owner.get_script().is_none() {
                script.bind().refusals().push((id, variables));
                continue;
            }
            for (name, value) in variables {
                owner.set(&name, &value);
            }
        }
    }
}

// What `owner`'s script variables hold.
fn read_script_variables(owner: &Gd<Object>) -> ScriptVariables {
    owner
        .get_property_list()
        .iter_shared()
        .filter(|property| {
            property
                .get("usage")
                .and_then(|usage| usage.try_to::<i64>().ok())
                .is_some_and(|usage| usage & PropertyUsageFlags::SCRIPT_VARIABLE.ord() as i64 != 0)
        })
        .filter_map(|property| property.get("name")?.try_to::<GString>().ok())
        .map(|name| StringName::from(&name))
        .map(|name| {
            let value = owner.get(&name);
            (name, value)
        })
        .collect()
}

impl RubyScript {
    /// The script of the file at `path`, holding `source`.
    pub fn from_source(path: &str, source: GString) -> Gd<Self> {
        let header = Header::from_source(path, &source.to_string(), &GameFiles.roots());
        let script = Gd::from_init_fn(|base| Self {
            base,
            header: Arc::new(header),
            ancestry: Cache::default(),
            methods: Mutex::default(),
            #[cfg(feature = "editor")]
            editor_state: editor::EditorState::default(),
            refusals: Mutex::default(),
            inner_scripts: Mutex::default(),
            source,
        });
        class_instance::attach(&script);
        script
    }

    /// Whether GDScript may call `method` on the script as a GDScript's
    /// static function: the class, or a class it inherits from, defines it
    /// on `self`.
    pub fn has_class_method(&self, method: &str) -> bool {
        self.lineage().has_class_method(method)
    }

    /// The constant the script's class is named by from the top level, as
    /// `Enemies::Boss`.
    pub fn class_constant(&self) -> String {
        let path = self.base().get_path().to_string();
        let (file, inner) = realm::split_inner_path(&path);
        std::iter::once(GameFiles.roots().name_by_path(file))
            .chain(inner.into_iter().map(str::to_owned))
            .collect::<Vec<_>>()
            .join("::")
    }

    /// The script standing for the class `names` spells inside the file's
    /// class, made the first time it is asked for, its path the file's
    /// followed by those names, as a GDScript's inner class is named.
    pub fn inner_script_by_names(&self, names: &[String]) -> Gd<RubyScript> {
        let path = realm::join_inner_path(&self.base().get_path().to_string(), names);
        let mut inner_scripts = self
            .inner_scripts
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        inner_scripts
            .entry(path)
            .or_insert_with_key(|path| {
                let mut script = Self::from_source(path, self.source.clone());
                script.take_over_path(path);
                script
            })
            .clone()
    }

    /// Takes the source its file holds now and reloads, as a GDScript reads
    /// its file again when the running game is told it changed. A file that
    /// cannot be read leaves the script as it was.
    pub fn reload_from_disk(mut script: Gd<Self>) {
        let path = script.get_path().to_string();
        let Ok(source) = FilesOnDisk.source_by_path(&path) else {
            return;
        };
        script.set_source_code(&source);
        script.reload();
    }

    fn refusals(&self) -> MutexGuard<'_, Vec<(InstanceId, ScriptVariables)>> {
        self.refusals.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn ancestry(&self) -> Result<Arc<Ancestry>, Break> {
        self.ancestry.fetch_ancestry(|| {
            let path = self.base().get_path().to_string();
            ancestry::trace_ancestry(&path, &self.header, &GameFiles)
        })
    }

    // What the script's header and `ancestry` define, shared with the
    // instances already made from the same two.
    fn methods_by_ancestry(&self, ancestry: &Arc<Ancestry>) -> Arc<Methods> {
        let mut kept = self.methods.lock().unwrap_or_else(PoisonError::into_inner);
        match &*kept {
            Some((header, kept_ancestry, methods))
                if Arc::ptr_eq(header, &self.header) && Arc::ptr_eq(kept_ancestry, ancestry) =>
            {
                Arc::clone(methods)
            }
            _ => {
                let methods = Arc::new(Methods::default());
                *kept = Some((
                    Arc::clone(&self.header),
                    Arc::clone(ancestry),
                    Arc::clone(&methods),
                ));
                methods
            }
        }
    }

    fn lineage(&self) -> Lineage<'_> {
        let path = self.base().get_path().to_string();
        Lineage::new(path, &self.header, self.ancestry().ok())
    }

    // The signals the class declared, its ancestors' included, nearest
    // first: what each file declared as it ran, or what its header writes
    // while it has not run.
    fn signals(&self) -> Vec<Signal> {
        self.lineage().collect_signals(&snapshot::latest())
    }

    // What the class declared for the editor, its ancestors' included and
    // in the order each class wrote it; a file that has not run has the
    // properties its header writes and no heading.
    fn members(&self) -> Vec<Member> {
        self.lineage().collect_members(&snapshot::latest())
    }

    // The property of that name the class exported, if it exported one.
    fn property_by_name(&self, name: &StringName) -> Option<Property> {
        self.lineage()
            .property_by_name(&snapshot::latest(), &name.to_string())
    }

    // The methods the file's class has: the ones its source defines, and the
    // ones it defined as it ran, which are the same but for metaprogramming's.
    fn methods(&self) -> BTreeSet<String> {
        let path = self.base().get_path().to_string();
        self.header
            .methods()
            .map(str::to_owned)
            .chain(snapshot::latest().methods_by_path(&path).iter().cloned())
            .collect()
    }

    // The engine class the file's class extends when it is a script
    // script, or why it is none.
    fn engine_class(&self) -> Result<StringName, Break> {
        read_engine_class(&*self.ancestry()?)
    }
}

// The engine class `ancestry` ends in, or why it ends in none.
fn read_engine_class(ancestry: &Ancestry) -> Result<StringName, Break> {
    let engine_class = ancestry.engine_class();
    game::is_engine_class(engine_class)
        .then(|| StringName::from(engine_class))
        .ok_or(Break::NoEngineClass)
}

#[godot_api]
impl IScriptExtension for RubyScript {
    fn editor_can_reload_from_file(&mut self) -> bool {
        true
    }

    // The editor makes an instance only of a tool whose source parses, as it
    // does of a GDScript, and never of one under a test directory, whose
    // files it never runs; every other node there is given a placeholder.
    // Godot asks this before making an instance, so a file that is no node
    // script is refused as the instance is made, where the refusal is
    // reported.
    fn can_instantiate(&self) -> bool {
        if !Engine::singleton().is_editor_hint() {
            return true;
        }
        let path = self.base().get_path().to_string();
        self.header.is_tool()
            && self.header.is_parsed()
            && !game::is_left_out_in_editor(&path, &settings::test_directories())
    }

    fn get_base_script(&self) -> Option<Gd<Script>> {
        let ancestry = self.ancestry().ok()?;
        let (path, _) = ancestry.files().first()?;
        ResourceLoader::singleton()
            .load_ex(path)
            .type_hint("Script")
            .done()?
            .try_cast::<Script>()
            .ok()
    }

    // The name the editor lists the script by, as a GDScript answers its
    // `class_name`; none for a script it does not announce.
    fn get_global_name(&self) -> StringName {
        let path = self.base().get_path().to_string();
        announcement::announcement_by_path(&path)
            .map(|announcement| StringName::from(&announcement.name))
            .unwrap_or_default()
    }

    // Whether `script` is this script or one its class inherits from, as a
    // GDScript answers of its own chain; a script of another language is
    // neither.
    fn inherits_script(&self, script: Gd<Script>) -> bool {
        script
            .try_cast::<RubyScript>()
            .is_ok_and(|script| self.lineage().has_file(&script.get_path().to_string()))
    }

    fn get_instance_base_type(&self) -> StringName {
        self.engine_class().unwrap_or_default()
    }

    // Only a script whose class extends an engine class makes an instance,
    // and only for an object of that class, as a GDScript refuses an object
    // its native type does not fit.
    unsafe fn instance_create_rawptr(&self, for_object: Gd<Object>) -> RawPtr<*mut c_void> {
        let path = self.base().get_path();
        let object_class = StringName::from(&for_object.get_class());
        let traced = self
            .ancestry()
            .and_then(|ancestry| Ok((read_engine_class(&ancestry)?, ancestry)));
        let (engine_class, ancestry) = match traced {
            Ok(traced) => traced,
            Err(broken) => {
                error!("{path} {broken}, so it cannot be the script of a {object_class}");
                return no_instance();
            }
        };
        if !ClassDb::singleton().is_parent_class(&object_class, &engine_class) {
            error!("{path} extends {engine_class}, so it cannot be the script of a {object_class}");
            return no_instance();
        }
        let language = self
            .get_language()
            .expect("the Ruby language outlives every Ruby script instance");
        let methods = self.methods_by_ancestry(&ancestry);
        let instance = RubyInstance::new(
            self.to_gd().upcast(),
            Arc::clone(&self.header),
            ancestry,
            methods,
            language,
            &for_object,
        );
        instance.into_godot()
    }

    unsafe fn placeholder_instance_create_rawptr(
        &self,
        for_object: Gd<Object>,
    ) -> RawPtr<*mut c_void> {
        editor::create_placeholder(self, for_object)
    }

    unsafe fn placeholder_erased_rawptr(&mut self, placeholder: RawPtr<*mut c_void>) {
        editor::erase_placeholder(self, placeholder);
    }

    fn instance_has(&self, _object: Gd<Object>) -> bool {
        false
    }

    fn has_source_code(&self) -> bool {
        true
    }

    fn get_source_code(&self) -> GString {
        self.source.clone()
    }

    fn set_source_code(&mut self, code: GString) {
        let path = self.base().get_path().to_string();
        let header = Header::from_source(&path, &code.to_string(), &GameFiles.roots());
        self.header = Arc::new(header);
        let inner_scripts: Vec<Gd<RubyScript>> = self
            .inner_scripts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .cloned()
            .collect();
        for mut inner in inner_scripts {
            inner.set_source_code(&code);
        }
        self.source = code;
        ancestry::expire();
    }

    // The file runs again in the realm at the next frame, where the objects
    // it made live on with their state, so there is no state to lose; the
    // editor's placeholders take what the source now exports, as a
    // GDScript's do as it reloads.
    fn reload(&mut self, _keep_state: bool) -> Error {
        realm::rerun(&self.base().get_path().to_string());
        self.update_exports();
        Error::OK
    }

    fn get_doc_class_name(&self) -> StringName {
        editor::read_doc_class_name(self)
    }

    fn get_documentation(&self) -> Array<AnyDictionary> {
        editor::describe_documentation(self)
    }

    // The icon the editor shows for a script the project's class list does
    // not carry, as one sharing its name, as a GDScript answers its `@icon`.
    fn get_class_icon_path(&self) -> GString {
        self.header.icon().unwrap_or_default().into()
    }

    fn has_method(&self, method: StringName) -> bool {
        let method = method.to_string();
        self.lineage().has_method(&snapshot::latest(), &method) || self.has_class_method(&method)
    }

    fn has_static_method(&self, method: StringName) -> bool {
        self.has_class_method(&method.to_string())
    }

    // A static method takes any arguments, since Ruby says nothing of them
    // before it runs; GDScript's analyzer checks a call against this.
    fn get_method_info(&self, method: StringName) -> AnyDictionary {
        let method = method.to_string();
        if self.has_class_method(&method) {
            describe_class_method(&method).upcast_any_dictionary()
        } else {
            VarDictionary::new().upcast_any_dictionary()
        }
    }

    fn is_tool(&self) -> bool {
        self.header.is_tool()
    }

    // Godot refuses an abstract script as an object's script before it asks
    // for an instance.
    fn is_abstract(&self) -> bool {
        self.header.is_abstract()
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn get_language(&self) -> Option<Gd<ScriptLanguage>> {
        language::language().map(Gd::upcast)
    }

    fn has_script_signal(&self, signal: StringName) -> bool {
        let signal = signal.to_string();
        self.signals()
            .iter()
            .any(|declared| declared.name == signal)
    }

    fn get_script_signal_list(&self) -> Array<AnyDictionary> {
        self.signals().iter().map(describe_signal).collect()
    }

    fn has_property_default_value(&self, property: StringName) -> bool {
        self.property_by_name(&property).is_some()
    }

    fn get_property_default_value(&self, property: StringName) -> Variant {
        self.property_by_name(&property)
            .map_or_else(Variant::nil, |property| property.default_value())
    }

    fn update_exports(&mut self) {
        editor::update_exports(self);
    }

    fn get_script_method_list(&self) -> Array<AnyDictionary> {
        let methods = self.methods();
        let class_methods = self.lineage().collect_class_methods();
        let described = methods.iter().map(|method| describe_name(method.as_str()));
        let class_described = class_methods
            .iter()
            .map(|method| describe_class_method(method));
        described
            .chain(class_described)
            .map(VarDictionary::upcast_any_dictionary)
            .collect()
    }

    fn get_script_property_list(&self) -> Array<AnyDictionary> {
        self.members().iter().map(describe_member).collect()
    }

    fn get_member_line(&self, _member: StringName) -> i32 {
        -1
    }

    fn get_constants(&self) -> AnyDictionary {
        VarDictionary::new().upcast_any_dictionary()
    }

    fn get_members(&self) -> Array<StringName> {
        Array::new()
    }

    // A placeholder of a script whose source does not parse keeps what a
    // scene wrote to the node, whatever the class is found to declare, so
    // the scene saves it back as a GDScript's does.
    fn is_placeholder_fallback_enabled(&self) -> bool {
        Engine::singleton().is_editor_hint() && !self.header.is_parsed()
    }

    // The methods the class lets peers call, its ancestors' included, as
    // GDScript's `@rpc` config has them, by method name.
    fn get_rpc_config(&self) -> Variant {
        let mut config = VarDictionary::new();
        for rpc in self.lineage().collect_rpcs(&snapshot::latest()) {
            config.set(&StringName::from(&rpc.method), &rpc.describe());
        }
        config.to_variant()
    }
}

// A declared signal as Godot reads it, which takes a method's shape: the
// signal's name, and a parameter for each value it carries.
fn describe_signal(signal: &Signal) -> AnyDictionary {
    let arguments: Array<AnyDictionary> =
        signal.parameters.iter().map(describe_parameter).collect();
    let mut info = describe_name(signal.name.as_str());
    info.set("args", &arguments);
    info.upcast_any_dictionary()
}

// What a class declared as Godot reads it, a property or the heading the
// properties after it are shown under.
fn describe_member(member: &Member) -> AnyDictionary {
    match member {
        Member::Property(property) => describe_property(property),
        Member::Heading(heading) => describe_heading(heading),
    }
}

// A heading as Godot reads it: the name it is headed with, the prefix it
// takes properties by, and the usage saying which kind of heading it is.
fn describe_heading(heading: &Heading) -> AnyDictionary {
    let mut info = describe_name(heading.name());
    info.set("type", VariantType::NIL.ord());
    info.set("hint_string", heading.hint_string());
    info.set("usage", heading.usage());
    info.upcast_any_dictionary()
}

// An exported property as Godot reads it: the name it is read and written
// by, the type the value it was declared with gave it, the hint the editor
// shows it with, and the usage of a script's own variable.
fn describe_property(property: &Property) -> AnyDictionary {
    let mut info = describe_name(property.name.as_str());
    info.set("type", property.kind.ord());
    info.set("hint", property.hint.ord());
    info.set("hint_string", property.hint_string.as_str());
    info.set("usage", property.usage());
    info.upcast_any_dictionary()
}

// A signal's parameter as Godot reads it, as GDScript describes a typed
// parameter: its type, the class an object of it is of, and the usage that
// reads nil as any type.
fn describe_parameter(parameter: &Parameter) -> AnyDictionary {
    let mut info = describe_name(parameter.name.as_str());
    info.set("type", parameter.kind.ord());
    info.set("class_name", parameter.class_name.as_str());
    info.set("usage", parameter.usage());
    info.upcast_any_dictionary()
}

// What Godot reads a method, a signal or a parameter by: its name, which for
// a method is all a Ruby class says about it.
fn describe_name(name: &str) -> VarDictionary {
    let mut info = VarDictionary::new();
    info.set("name", name);
    info
}

// A method the class defines on `self`, as Godot reads a static function's:
// any arguments, and an answer of any type.
fn describe_class_method(name: &str) -> VarDictionary {
    let mut answer = VarDictionary::new();
    answer.set("type", VariantType::NIL.ord());
    answer.set("usage", PropertyUsageFlags::NIL_IS_VARIANT.ord() as i64);
    let mut info = describe_name(name);
    let flags = MethodFlags::NORMAL | MethodFlags::STATIC | MethodFlags::VARARG;
    info.set("flags", flags.ord() as i64);
    info.set("args", &VarArray::new());
    info.set("default_args", &VarArray::new());
    info.set("return", &answer);
    info
}

// No instance: the object is left without a script instance.
fn no_instance() -> RawPtr<*mut c_void> {
    // SAFETY: a null pointer is how a script tells Godot it made no instance.
    unsafe { RawPtr::new(std::ptr::null_mut()) }
}
