use std::collections::BTreeSet;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
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
use rustc_hash::FxHashMap;

use crate::ancestry::{self, Ancestry, Break, Cache, Lineage};
use crate::game::{self, FilesOnDisk, GameFiles};
use crate::header::Header;
use crate::instance::{self, Methods, RubyInstance};
use crate::language;
use crate::log::GodotLog;
use crate::realm::{self, Files};
use crate::settings;
use crate::snapshot::{self, Heading, Member, Parameter, Property, Signal};
use crate::{announcement, documentation, error};

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
    // The placeholders Godot made of the script in the editor and has not
    // erased, told again what the class exports whenever that changes.
    placeholders: Mutex<Vec<Placeholder>>,
    // What they were last told, as `Exports::digest` has it: Godot asks for
    // the exports again for every property of a scene it saves, and telling
    // a placeholder makes the editor list its properties anew.
    last_exports_digest: Mutex<Option<(u32, u32)>>,
    // Whether the script is a template made for a new file, which learns its
    // path only as it is first saved. Taken without binding the script
    // mutably, since another thread may be reading it then.
    is_from_template: AtomicBool,
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
            methods: Mutex::default(),
            placeholders: Mutex::default(),
            last_exports_digest: Mutex::default(),
            is_from_template: AtomicBool::new(false),
            refusals: Mutex::default(),
            inner_scripts: Mutex::default(),
            source,
        })
    }

    /// The script a template made for a new file, holding `source`.
    pub fn from_template(source: GString) -> Gd<Self> {
        let script = Self::from_source("", source);
        script
            .bind()
            .is_from_template
            .store(true, Ordering::Relaxed);
        script
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

    /// Whether the script is a template made for a new file, which it stops
    /// being once asked.
    pub fn take_template_mark(&self) -> bool {
        self.is_from_template.swap(false, Ordering::Relaxed)
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

    fn refusals(&self) -> MutexGuard<'_, Vec<(InstanceId, ScriptVariables)>> {
        self.refusals.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn placeholders(&self) -> MutexGuard<'_, Vec<Placeholder>> {
        self.placeholders
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
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

    // The name the script's page goes by in the editor's help and the page
    // it inherits from: the names the editor lists the class and its base
    // by, or for a script it does not announce, its path quoted, as a
    // GDScript without `class_name` is named, and its engine class.
    fn read_doc_names(&self) -> (String, String) {
        let path = self.base().get_path().to_string();
        match announcement::announcement_by_path(&path) {
            Ok(announcement) => (announcement.name, announcement.base),
            Err(_) => (
                format!("\"{}\"", path.trim_start_matches("res://")),
                self.ancestry()
                    .map(|ancestry| ancestry.engine_class().to_owned())
                    .unwrap_or_default(),
            ),
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

    // The properties the class exported, its ancestors' included, nearest
    // first: what each file declared as it ran, or what its header writes
    // while it has not run.
    fn properties(&self) -> Vec<Property> {
        self.lineage().collect_properties(&snapshot::latest())
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

    // What the class exports and the value each was declared with, which is
    // all the editor has to show a placeholder while no object of the class
    // exists.
    fn exports(&self) -> Exports {
        let mut values = VarDictionary::new();
        for property in self.properties() {
            values.set(&StringName::from(&property.name), &property.default_value());
        }
        Exports {
            properties: self.members().iter().map(describe_member).collect(),
            values,
        }
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
        StringName::from(&self.read_doc_names().0)
    }

    // The class's page in the editor's help, from what the source's comments
    // say of the class and its members, as a GDScript's `##` comments give
    // its page.
    fn get_documentation(&self) -> Array<AnyDictionary> {
        let path = self.base().get_path().to_string();
        let documentation =
            documentation::read_documentation(&self.source.to_string(), &self.header);
        let (name, inherits) = self.read_doc_names();
        let describe = |entries: &[documentation::Entry]| -> Array<AnyDictionary> {
            entries.iter().map(describe_entry).collect()
        };
        let class = vdict! {
            "name" => name,
            "inherits" => inherits,
            "brief_description" => documentation.brief,
            "description" => documentation.description,
            "methods" => &describe(&documentation.methods),
            "properties" => &describe(&documentation.properties),
            "signals" => &describe(&documentation.signals),
            "is_script_doc" => true,
            "script_path" => path,
        };
        std::iter::once(class.upcast_any_dictionary()).collect()
    }

    // The icon the editor shows for a script the project's class list does
    // not carry, as one sharing its name, as a GDScript answers its `@icon`.
    fn get_class_icon_path(&self) -> GString {
        self.header.icon().unwrap_or_default().into()
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
        self.signals().iter().map(describe_signal).collect()
    }

    fn has_property_default_value(&self, property: StringName) -> bool {
        self.property_by_name(&property).is_some()
    }

    fn get_property_default_value(&self, property: StringName) -> Variant {
        self.property_by_name(&property)
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
            .map(|method| describe_name(method.as_str()).upcast_any_dictionary())
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

    fn get_rpc_config(&self) -> Variant {
        Variant::nil()
    }
}

// A member of a class's page in the editor's help.
fn describe_entry(entry: &documentation::Entry) -> AnyDictionary {
    vdict! {
        "name" => entry.name.as_str(),
        "description" => entry.description.as_str(),
    }
    .upcast_any_dictionary()
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

// No instance: the object is left without a script instance.
fn no_instance() -> RawPtr<*mut c_void> {
    // SAFETY: a null pointer is how a script tells Godot it made no instance.
    unsafe { RawPtr::new(std::ptr::null_mut()) }
}
