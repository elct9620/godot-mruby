use std::ffi::c_void;

use godot::classes::{IScriptExtension, Object, Script, ScriptExtension, ScriptLanguage};
use godot::global::Error;
use godot::meta::conv::RawPtr;
use godot::prelude::*;

use crate::language;

/// The script a `.rb` file loads as, the way a `.gd` file loads as a `GDScript`.
///
/// It holds the file's source and answers Godot's questions without running
/// any Ruby: a loaded script only becomes code when something enters it.
#[derive(GodotClass)]
#[class(base = ScriptExtension, init, tool)]
pub struct RubyScript {
    base: Base<ScriptExtension>,
    source: GString,
}

impl RubyScript {
    pub fn from_source(source: GString) -> Gd<Self> {
        Gd::from_init_fn(|base| Self { base, source })
    }
}

#[godot_api]
impl IScriptExtension for RubyScript {
    fn editor_can_reload_from_file(&mut self) -> bool {
        true
    }

    fn can_instantiate(&self) -> bool {
        false
    }

    fn get_base_script(&self) -> Option<Gd<Script>> {
        None
    }

    fn get_global_name(&self) -> StringName {
        StringName::default()
    }

    fn inherits_script(&self, _script: Gd<Script>) -> bool {
        false
    }

    fn get_instance_base_type(&self) -> StringName {
        StringName::from("Node")
    }

    unsafe fn instance_create_rawptr(&self, _for_object: Gd<Object>) -> RawPtr<*mut c_void> {
        // SAFETY: a null instance is what a script that cannot instantiate returns.
        unsafe { RawPtr::new(std::ptr::null_mut()) }
    }

    unsafe fn placeholder_instance_create_rawptr(
        &self,
        _for_object: Gd<Object>,
    ) -> RawPtr<*mut c_void> {
        // SAFETY: a null placeholder leaves the object without a script instance.
        unsafe { RawPtr::new(std::ptr::null_mut()) }
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

    fn has_method(&self, _method: StringName) -> bool {
        false
    }

    fn has_static_method(&self, _method: StringName) -> bool {
        false
    }

    fn get_method_info(&self, _method: StringName) -> AnyDictionary {
        VarDictionary::new().upcast_any_dictionary()
    }

    fn is_tool(&self) -> bool {
        false
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn get_language(&self) -> Option<Gd<ScriptLanguage>> {
        language::registered().map(Gd::upcast)
    }

    fn has_script_signal(&self, _signal: StringName) -> bool {
        false
    }

    fn get_script_signal_list(&self) -> Array<AnyDictionary> {
        Array::new()
    }

    fn has_property_default_value(&self, _property: StringName) -> bool {
        false
    }

    fn get_property_default_value(&self, _property: StringName) -> Variant {
        Variant::nil()
    }

    fn update_exports(&mut self) {}

    fn get_script_method_list(&self) -> Array<AnyDictionary> {
        Array::new()
    }

    fn get_script_property_list(&self) -> Array<AnyDictionary> {
        Array::new()
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
