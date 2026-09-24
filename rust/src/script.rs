use std::collections::BTreeSet;
use std::ffi::c_void;
use std::sync::{Arc, OnceLock};

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

use crate::ancestry::{self, Ancestry, Broken, Lineage};
use crate::game::GameFiles;
use crate::header::Header;
use crate::instance::RubyInstance;
use crate::language;
use crate::realm::Files;
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
    // since loading a script must not load the scripts it inherits from.
    ancestry: OnceLock<Result<Arc<Ancestry>, Broken>>,
}

impl RubyScript {
    /// The script of the file at `path`, holding `source`.
    pub fn from_source(path: &str, source: GString) -> Gd<Self> {
        let header = Header::read(path, &source.to_string(), &GameFiles.roots());
        Gd::from_init_fn(|base| Self {
            base,
            header: Arc::new(header),
            ancestry: OnceLock::new(),
            source,
        })
    }

    fn ancestry(&self) -> &Result<Arc<Ancestry>, Broken> {
        self.ancestry.get_or_init(|| {
            let path = self.base().get_path().to_string();
            ancestry::read(&path, &self.header, &GameFiles).map(Arc::new)
        })
    }

    fn lineage(&self) -> Lineage<'_> {
        let path = self.base().get_path().to_string();
        let ancestry = self.ancestry().as_ref().ok().map(Arc::as_ref);
        Lineage::new(path, &self.header, ancestry)
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

    // Tells a placeholder what the class exports and the value each was
    // declared with, which is all the editor has to show it while no object
    // of the class exists.
    fn fill(&self, placeholder: sys::GDExtensionScriptInstancePtr) {
        let properties: Array<AnyDictionary> = self.members().iter().map(member_info).collect();
        let mut values = VarDictionary::new();
        for property in self.properties() {
            values.set(&StringName::from(&property.name), &property.default_value());
        }
        // SAFETY: the placeholder is one the engine made for this script,
        // and both values outlive the call.
        unsafe {
            sys::interface_fn!(placeholder_script_instance_update)(
                placeholder,
                properties.sys(),
                values.sys(),
            );
        }
    }

    // The engine node class the file's class extends when it is a node
    // script, or why it is none.
    fn node_class(&self) -> Result<StringName, Broken> {
        let ancestry = self.ancestry().as_ref().map_err(Clone::clone)?;
        let engine_class = ancestry.engine_class();
        bridge::is_node_class(engine_class)
            .then(|| StringName::from(engine_class))
            .ok_or(Broken::NoEngineClass)
    }
}

#[godot_api]
impl IScriptExtension for RubyScript {
    fn editor_can_reload_from_file(&mut self) -> bool {
        true
    }

    // Scripts only run in a game; the editor gets no instance. Godot asks
    // this before making an instance, so a file that is no node script is
    // refused as the instance is made, where the refusal is reported.
    fn can_instantiate(&self) -> bool {
        !Engine::singleton().is_editor_hint()
    }

    fn get_base_script(&self) -> Option<Gd<Script>> {
        let (path, _) = self.ancestry().as_ref().ok()?.files().first()?;
        ResourceLoader::singleton()
            .load_ex(path)
            .type_hint("Script")
            .done()?
            .try_cast::<Script>()
            .ok()
    }

    fn get_global_name(&self) -> StringName {
        StringName::default()
    }

    fn inherits_script(&self, _script: Gd<Script>) -> bool {
        false
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
            self.ancestry()
                .as_ref()
                .map(Arc::clone)
                .expect("a node script has an ancestry"),
            language,
            &for_object,
        );
        instance.into_godot()
    }

    // The editor makes no instance, so Godot asks for its own placeholder
    // and shows the node what that carries. The file never runs there, so
    // what it carries is the header's answer.
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
        self.fill(placeholder);
        // SAFETY: the pointer is the placeholder Godot just made.
        unsafe { RawPtr::new(placeholder.cast::<c_void>()) }
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
        let header = Header::read(&path, &code.to_string(), &GameFiles.roots());
        self.header = Arc::new(header);
        self.ancestry = OnceLock::new();
        self.source = code;
    }

    fn reload(&mut self, _keep_state: bool) -> Error {
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
        language::registered_language().map(Gd::upcast)
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

    fn update_exports(&mut self) {}

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

    fn is_placeholder_fallback_enabled(&self) -> bool {
        false
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
