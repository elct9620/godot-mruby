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
use godot::register::info::PropertyUsageFlags;
use godot::sys::{self, GodotFfi};

use crate::ancestry::{self, Ancestry, Break, Cache, Lineage};
use crate::announcement::Project;
use crate::game::{self, FilesOnDisk, GameFiles};
use crate::header::Header;
use crate::instance::RubyInstance;
use crate::language;
use crate::log::GodotLog;
use crate::realm::{self, Files};
use crate::settings;
use crate::snapshot::{self, Heading, Member, Property, Signal};
use crate::{bridge, error};

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
    // The placeholders Godot made of the script in the editor and has not
    // erased, told again what the class exports whenever that changes.
    placeholders: Mutex<Vec<Placeholder>>,
    // What they were last told, as `Exports::digest` has it: Godot asks for
    // the exports again for every property of a scene it saves, and telling
    // a placeholder makes the editor list its properties anew.
    last_exports_digest: Mutex<Option<(u32, u32)>>,
    // Whether the script is a template made for a new file, which learns its
    // path only as it is first saved.
    is_from_template: bool,
}

/// What a placeholder is told of the class: what it exports, and the value
/// each was declared with.
struct Exports {
    properties: Array<AnyDictionary>,
    values: VarDictionary,
}

impl Exports {
    // A digest of what it tells, which differs whenever that does.
    fn digest(&self) -> (u32, u32) {
        (
            self.properties.to_variant().hash_u32(),
            self.values.to_variant().hash_u32(),
        )
    }

    // Tells `placeholder`, which Godot answers by asking the script about the
    // class again.
    fn tell(&self, placeholder: sys::GDExtensionScriptInstancePtr) {
        // SAFETY: the placeholder is one the engine made for this script and
        // has not erased, and both values outlive the call.
        unsafe {
            sys::interface_fn!(placeholder_script_instance_update)(
                placeholder,
                self.properties.sys(),
                self.values.sys(),
            );
        }
    }
}

// The scripts holding placeholders, which are told what their classes
// declare whenever a file runs in the editor's realm.
static PLACEHOLDER_SCRIPTS: Mutex<Vec<InstanceId>> = Mutex::new(Vec::new());
// The paths of scripts Godot made placeholders of since the last frame,
// whose files the editor's realm runs then.
static PENDING_PATHS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Runs in the game's realm the file of each script Godot made a
/// placeholder of since the last frame, unless it is one the editor leaves
/// out, then tells every placeholder what its class declares now, as it
/// does once files have `run_again`: the editor learns a class's hints,
/// headings and the members only running defines as it would a GDScript's.
/// Placeholders exist only in the editor, and the extension calls this
/// every frame.
pub fn run_placed(run_again: bool) {
    let paths = std::mem::take(&mut *PENDING_PATHS.lock().unwrap_or_else(PoisonError::into_inner));
    if paths.is_empty() {
        if run_again {
            tell_placed();
        }
        return;
    }
    let test_directories = settings::test_directories();
    for path in paths {
        if game::is_left_out_in_editor(&path, &test_directories) {
            continue;
        }
        if let Err(error) = realm::enter(|realm| realm.run(&path)) {
            error.write(&GodotLog);
        }
    }
    tell_placed();
}

// Tells every script holding placeholders to tell them what its class
// declares, which each does only when that changed.
fn tell_placed() {
    let placed = PLACEHOLDER_SCRIPTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    for id in placed {
        if let Ok(mut script) = Gd::<RubyScript>::try_from_instance_id(id) {
            script.bind_mut().update_exports();
        }
    }
}

/// A placeholder Godot made of a script, which only Godot frees.
struct Placeholder(sys::GDExtensionScriptInstancePtr);

// SAFETY: the pointer is only handed back to Godot, which keeps the
// placeholder alive until it tells the script it erased it.
unsafe impl Send for Placeholder {}

impl RubyScript {
    /// The script of the file at `path`, holding `source`.
    pub fn from_source(path: &str, source: GString) -> Gd<Self> {
        let header = Header::from_source(path, &source.to_string(), &GameFiles.roots());
        Gd::from_init_fn(|base| Self {
            base,
            header: Arc::new(header),
            ancestry: Cache::default(),
            placeholders: Mutex::default(),
            last_exports_digest: Mutex::default(),
            is_from_template: false,
            source,
        })
    }

    /// The script a template made for a new file, holding `source`.
    pub fn from_template(source: GString) -> Gd<Self> {
        let mut script = Self::from_source("", source);
        script.bind_mut().is_from_template = true;
        script
    }

    /// Whether the script is a template made for a new file, which it stops
    /// being once asked.
    pub fn take_template_mark(&mut self) -> bool {
        std::mem::take(&mut self.is_from_template)
    }

    /// Takes the source its file holds now and reloads, as a GDScript reads
    /// its file again when the running game is told it changed. A file that
    /// cannot be read leaves the script as it was.
    pub fn reload_from_disk(mut script: Gd<Self>) {
        let path = script.get_path().to_string();
        let Ok(source) = FilesOnDisk.source(&path) else {
            return;
        };
        script.set_source_code(&source);
        script.reload();
    }

    // Keeps the script among those holding placeholders, and its file, if it
    // has one, for the editor's realm to run at the next frame.
    fn place(&self) {
        let id = self.base().instance_id();
        let mut placed = PLACEHOLDER_SCRIPTS
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if !placed.contains(&id) {
            placed.push(id);
        }
        let path = self.base().get_path().to_string();
        let mut unrun = PENDING_PATHS.lock().unwrap_or_else(PoisonError::into_inner);
        if !path.is_empty() && !unrun.contains(&path) {
            unrun.push(path);
        }
    }

    fn placeholders(&self) -> MutexGuard<'_, Vec<Placeholder>> {
        self.placeholders
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn ancestry(&self) -> Result<Arc<Ancestry>, Break> {
        self.ancestry.ancestry(|| {
            let path = self.base().get_path().to_string();
            ancestry::ancestry_of(&path, &self.header, &GameFiles)
        })
    }

    fn lineage(&self) -> Lineage<'_> {
        let path = self.base().get_path().to_string();
        Lineage::new(path, &self.header, self.ancestry().ok())
    }

    // The signals the class has, its ancestors' included, nearest first:
    // what each file declared as it ran, or what its header writes while it
    // has not run, so a scene's connection is made before anything runs.
    fn signals(&self) -> Vec<Signal> {
        let snapshot = snapshot::latest();
        let mut signals: Vec<Signal> = Vec::new();
        for (path, header) in self.lineage().files() {
            let declared = if snapshot.has_run(path) {
                snapshot.signals(path)
            } else {
                header.signals()
            };
            for signal in declared {
                if !signals.iter().any(|kept| kept.name == signal.name) {
                    signals.push(signal.clone());
                }
            }
        }
        signals
    }

    // The properties the class exported, its ancestors' included, nearest
    // first: what each file declared as it ran, or what its header writes
    // while it has not run.
    fn properties(&self) -> Vec<Property> {
        self.lineage().properties(&snapshot::latest())
    }

    // What the class declared for the editor, its ancestors' included and
    // in the order each class wrote it; a file that has not run has the
    // properties its header writes and no heading.
    fn members(&self) -> Vec<Member> {
        self.lineage().members(&snapshot::latest())
    }

    // The property of that name the class exported, if it exported one.
    fn property(&self, name: &StringName) -> Option<Property> {
        let name = name.to_string();
        self.properties()
            .into_iter()
            .find(|property| property.name == name)
    }

    // The methods the file's class has: the ones its source defines, and the
    // ones it defined as it ran, which are the same but for metaprogramming's.
    fn methods(&self) -> BTreeSet<String> {
        let path = self.base().get_path().to_string();
        self.header
            .methods()
            .map(str::to_owned)
            .chain(snapshot::latest().methods(&path).iter().cloned())
            .collect()
    }

    // What the class exports and the value each was declared with, which is
    // all the editor has to show a placeholder while no object of the class
    // exists.
    fn exports(&self) -> Exports {
        let mut values = VarDictionary::new();
        for property in self.properties() {
            values.set(&StringName::from(&property.name), &property.default_value());
        }
        Exports {
            properties: self.members().iter().map(member_info).collect(),
            values,
        }
    }

    // The engine node class the file's class extends when it is a node
    // script, or why it is none.
    fn node_class(&self) -> Result<StringName, Break> {
        let ancestry = self.ancestry()?;
        let engine_class = ancestry.engine_class();
        bridge::is_node_class(engine_class)
            .then(|| StringName::from(engine_class))
            .ok_or(Break::NoEngineClass)
    }
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
        let test_directories = settings::test_directories();
        Project::new(
            &FilesOnDisk,
            &test_directories,
            settings::template_directory(),
            &bridge::is_node_class,
        )
        .announcement(&path)
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
        self.node_class().unwrap_or_default()
    }

    // Only a node script makes an instance, and only for a node of the engine
    // class it extends, as a GDScript refuses an object its native type does
    // not fit.
    unsafe fn instance_create_rawptr(&self, for_object: Gd<Object>) -> RawPtr<*mut c_void> {
        let path = self.base().get_path();
        let engine_class = match self.node_class() {
            Ok(engine_class) => engine_class,
            Err(broken) => {
                error!("{path} {broken}, so it cannot be a node's script");
                return no_instance();
            }
        };
        let object_class = StringName::from(&for_object.get_class());
        if !ClassDb::singleton().is_parent_class(&object_class, &engine_class) {
            error!("{path} extends {engine_class}, so it cannot be the script of a {object_class}");
            return no_instance();
        }
        let language = self
            .get_language()
            .expect("the Ruby language outlives every Ruby script instance");
        let instance = RubyInstance::new(
            self.to_gd().upcast(),
            Arc::clone(&self.header),
            self.ancestry().expect("a node script has an ancestry"),
            language,
            &for_object,
        );
        instance.into_godot()
    }

    // The editor makes no instance, so Godot asks for its own placeholder
    // and shows the node what that carries: the header's answer at first,
    // then what the class declares once the editor's realm has run the file.
    unsafe fn placeholder_instance_create_rawptr(
        &self,
        for_object: Gd<Object>,
    ) -> RawPtr<*mut c_void> {
        let Some(language) = self.get_language() else {
            return no_instance();
        };
        // SAFETY: the interface is initialized while the extension runs, and
        // Godot frees the placeholder with the object it was made for.
        let placeholder = unsafe {
            sys::interface_fn!(placeholder_script_instance_create)(
                language.obj_sys(),
                self.to_gd().obj_sys(),
                for_object.obj_sys(),
            )
        };
        if self.header.is_parsed() {
            self.exports().tell(placeholder);
        }
        self.placeholders().push(Placeholder(placeholder));
        self.place();
        // SAFETY: the pointer is the placeholder Godot just made.
        unsafe { RawPtr::new(placeholder.cast::<c_void>()) }
    }

    unsafe fn placeholder_erased_rawptr(&mut self, placeholder: RawPtr<*mut c_void>) {
        let erased = placeholder.ptr().cast();
        let mut placeholders = self.placeholders();
        placeholders.retain(|kept| kept.0 != erased);
        if placeholders.is_empty() {
            let id = self.base().instance_id();
            PLACEHOLDER_SCRIPTS
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .retain(|placed| *placed != id);
        }
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
        StringName::default()
    }

    fn get_documentation(&self) -> Array<AnyDictionary> {
        Array::new()
    }

    fn has_method(&self, method: StringName) -> bool {
        self.lineage()
            .has_method(&snapshot::latest(), &method.to_string())
    }

    fn has_static_method(&self, _method: StringName) -> bool {
        false
    }

    fn get_method_info(&self, _method: StringName) -> AnyDictionary {
        VarDictionary::new().upcast_any_dictionary()
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
        self.signals().iter().map(signal_info).collect()
    }

    fn has_property_default_value(&self, property: StringName) -> bool {
        self.property(&property).is_some()
    }

    fn get_property_default_value(&self, property: StringName) -> Variant {
        self.property(&property)
            .map_or_else(Variant::nil, |property| property.default_value())
    }

    // Godot asks the script about the class as each placeholder is told, so
    // the script is let go of while they are. A source that does not parse
    // tells them nothing, as a GDScript's does not: telling a placeholder
    // drops what it kept of the scene beyond what the class declares.
    fn update_exports(&mut self) {
        if !self.header.is_parsed() {
            return;
        }
        let exported = self.exports();
        let digest = Some(exported.digest());
        let mut last_exports_digest = self
            .last_exports_digest
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if *last_exports_digest == digest {
            return;
        }
        *last_exports_digest = digest;
        drop(last_exports_digest);
        let placeholders: Vec<_> = self.placeholders().iter().map(|kept| kept.0).collect();
        let _released = self.base_mut();
        for placeholder in placeholders {
            exported.tell(placeholder);
        }
    }

    fn get_script_method_list(&self) -> Array<AnyDictionary> {
        self.methods()
            .iter()
            .map(|method| name_info(method.as_str()).upcast_any_dictionary())
            .collect()
    }

    fn get_script_property_list(&self) -> Array<AnyDictionary> {
        self.members().iter().map(member_info).collect()
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

    fn get_rpc_config(&self) -> Variant {
        Variant::nil()
    }
}

// A declared signal as Godot reads it, which takes a method's shape: the
// signal's name, and a parameter for each value it carries, of any type as
// an untyped GDScript signal's parameters are.
fn signal_info(signal: &Signal) -> AnyDictionary {
    let arguments: Array<AnyDictionary> = signal
        .parameters
        .iter()
        .map(|parameter| parameter_info(parameter))
        .collect();
    let mut info = name_info(signal.name.as_str());
    info.set("args", &arguments);
    info.upcast_any_dictionary()
}

// What a class declared as Godot reads it, a property or the heading the
// properties after it are shown under.
fn member_info(member: &Member) -> AnyDictionary {
    match member {
        Member::Property(property) => property_info(property),
        Member::Heading(heading) => heading_info(heading),
    }
}

// A heading as Godot reads it: the name it is headed with, the prefix it
// takes properties by, and the usage saying which kind of heading it is.
fn heading_info(heading: &Heading) -> AnyDictionary {
    let mut info = name_info(heading.name());
    info.set("type", VariantType::NIL.ord());
    info.set("hint_string", heading.hint_string());
    info.set("usage", heading.usage());
    info.upcast_any_dictionary()
}

// An exported property as Godot reads it: the name it is read and written
// by, the type the value it was declared with gave it, the hint the editor
// shows it with, and the usage of a script's own variable, which the editor
// shows and a scene stores.
fn property_info(property: &Property) -> AnyDictionary {
    let mut info = name_info(property.name.as_str());
    info.set("type", property.kind.ord());
    info.set("hint", property.hint.ord());
    info.set("hint_string", property.hint_string.as_str());
    info.set(
        "usage",
        PropertyUsageFlags::from_ord(
            PropertyUsageFlags::DEFAULT.ord() | PropertyUsageFlags::SCRIPT_VARIABLE.ord(),
        ),
    );
    info.upcast_any_dictionary()
}

fn parameter_info(name: &str) -> AnyDictionary {
    let mut info = name_info(name);
    info.set("type", VariantType::NIL.ord());
    info.set("usage", PropertyUsageFlags::NIL_IS_VARIANT);
    info.upcast_any_dictionary()
}

// What Godot reads a method, a signal or a parameter by: its name, which for
// a method is all a Ruby class says about it.
fn name_info(name: &str) -> VarDictionary {
    let mut info = VarDictionary::new();
    info.set("name", name);
    info
}

// No instance: the object is left without a script instance.
fn no_instance() -> RawPtr<*mut c_void> {
    // SAFETY: a null pointer is how a script tells Godot it made no instance.
    unsafe { RawPtr::new(std::ptr::null_mut()) }
}
