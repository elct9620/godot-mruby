use std::ffi::c_void;
use std::sync::Mutex;

use godot::classes::native::ScriptLanguageExtensionProfilingInfo;
use godot::classes::{Engine, IScriptLanguageExtension, Object, Script, ScriptLanguageExtension};
use godot::global::Error;
use godot::meta::conv::RawPtr;
use godot::prelude::*;

use crate::script::RubyScript;

/// The Ruby script language, registered with the engine for `.rb` files.
///
/// Every virtual answers from constants: Godot asks the language for a
/// backtrace while it prints an error, from whichever thread raised it, so
/// nothing here may wait on anything.
#[derive(GodotClass)]
#[class(base = ScriptLanguageExtension, init, tool)]
pub struct RubyLanguage {
    base: Base<ScriptLanguageExtension>,
}

static REGISTERED: Mutex<Option<InstanceId>> = Mutex::new(None);

pub fn register() {
    let language = RubyLanguage::new_alloc();
    Engine::singleton().register_script_language(&language);
    *REGISTERED.lock().unwrap() = Some(language.instance_id());
}

pub fn unregister() {
    if let Some(id) = REGISTERED.lock().unwrap().take() {
        let language = Gd::<RubyLanguage>::from_instance_id(id);
        Engine::singleton().unregister_script_language(&language);
        language.free();
    }
}

/// The registered language, which every script reports as its own.
pub fn registered() -> Option<Gd<RubyLanguage>> {
    REGISTERED.lock().unwrap().map(Gd::from_instance_id)
}

fn empty_dictionary() -> AnyDictionary {
    VarDictionary::new().upcast_any_dictionary()
}

#[godot_api]
impl IScriptLanguageExtension for RubyLanguage {
    fn get_name(&self) -> GString {
        "Ruby".into()
    }

    fn init_ext(&mut self) {}

    fn get_type(&self) -> GString {
        RubyScript::class_id().to_gstring()
    }

    fn get_extension(&self) -> GString {
        "rb".into()
    }

    fn finish(&mut self) {}

    fn get_reserved_words(&self) -> PackedStringArray {
        PackedStringArray::new()
    }

    fn is_control_flow_keyword(&self, _keyword: GString) -> bool {
        false
    }

    fn get_comment_delimiters(&self) -> PackedStringArray {
        PackedStringArray::new()
    }

    fn get_string_delimiters(&self) -> PackedStringArray {
        PackedStringArray::new()
    }

    fn make_template(
        &self,
        _template: GString,
        _class_name: GString,
        _base_class_name: GString,
    ) -> Option<Gd<Script>> {
        None
    }

    fn get_built_in_templates(&self, _object: StringName) -> Array<AnyDictionary> {
        Array::new()
    }

    fn is_using_templates(&mut self) -> bool {
        false
    }

    // Sources are not checked, so each is reported valid.
    fn validate(
        &self,
        _script: GString,
        _path: GString,
        _validate_functions: bool,
        _validate_errors: bool,
        _validate_warnings: bool,
        _validate_safe_lines: bool,
    ) -> AnyDictionary {
        vdict! { "valid" => true }.upcast_any_dictionary()
    }

    fn validate_path(&self, _path: GString) -> GString {
        GString::new()
    }

    fn create_script(&self) -> Option<Gd<Object>> {
        Some(RubyScript::new_gd().upcast())
    }

    fn supports_builtin_mode(&self) -> bool {
        false
    }

    fn supports_documentation(&self) -> bool {
        false
    }

    fn can_inherit_from_file(&self) -> bool {
        false
    }

    fn find_function(&self, _function: GString, _code: GString) -> i32 {
        -1
    }

    fn make_function(
        &self,
        _class_name: GString,
        _function_name: GString,
        _function_args: PackedStringArray,
    ) -> GString {
        GString::new()
    }

    fn can_make_function(&self) -> bool {
        false
    }

    fn open_in_external_editor(
        &mut self,
        _script: Option<Gd<Script>>,
        _line: i32,
        _column: i32,
    ) -> Error {
        Error::ERR_UNAVAILABLE
    }

    fn overrides_external_editor(&mut self) -> bool {
        false
    }

    // Godot reads these keys unconditionally and logs an error when one is missing.
    fn complete_code(
        &self,
        _code: GString,
        _path: GString,
        _owner: Option<Gd<Object>>,
    ) -> AnyDictionary {
        vdict! { "result" => Error::ERR_UNAVAILABLE, "force" => false, "call_hint" => "" }
            .upcast_any_dictionary()
    }

    // Godot reads these keys unconditionally and logs an error when one is missing.
    fn lookup_code(
        &self,
        _code: GString,
        _symbol: GString,
        _path: GString,
        _owner: Option<Gd<Object>>,
    ) -> AnyDictionary {
        vdict! { "result" => Error::ERR_UNAVAILABLE, "type" => 0 }.upcast_any_dictionary()
    }

    fn auto_indent_code(&self, code: GString, _from_line: i32, _to_line: i32) -> GString {
        code
    }

    fn add_global_constant(&mut self, _name: StringName, _value: Variant) {}

    fn add_named_global_constant(&mut self, _name: StringName, _value: Variant) {}

    fn remove_named_global_constant(&mut self, _name: StringName) {}

    fn thread_enter(&mut self) {}

    fn thread_exit(&mut self) {}

    fn debug_get_error(&self) -> GString {
        GString::new()
    }

    fn debug_get_stack_level_count(&self) -> i32 {
        0
    }

    fn debug_get_stack_level_line(&self, _level: i32) -> i32 {
        -1
    }

    fn debug_get_stack_level_function(&self, _level: i32) -> GString {
        GString::new()
    }

    fn debug_get_stack_level_source(&self, _level: i32) -> GString {
        GString::new()
    }

    fn debug_get_stack_level_locals(
        &mut self,
        _level: i32,
        _max_subitems: i32,
        _max_depth: i32,
    ) -> AnyDictionary {
        empty_dictionary()
    }

    fn debug_get_stack_level_members(
        &mut self,
        _level: i32,
        _max_subitems: i32,
        _max_depth: i32,
    ) -> AnyDictionary {
        empty_dictionary()
    }

    unsafe fn debug_get_stack_level_instance_rawptr(&mut self, _level: i32) -> RawPtr<*mut c_void> {
        // SAFETY: a null instance is how a language says the level has none.
        unsafe { RawPtr::new(std::ptr::null_mut()) }
    }

    fn debug_get_globals(&mut self, _max_subitems: i32, _max_depth: i32) -> AnyDictionary {
        empty_dictionary()
    }

    fn debug_parse_stack_level_expression(
        &mut self,
        _level: i32,
        _expression: GString,
        _max_subitems: i32,
        _max_depth: i32,
    ) -> GString {
        GString::new()
    }

    fn debug_get_current_stack_info(&mut self) -> Array<AnyDictionary> {
        Array::new()
    }

    fn reload_all_scripts(&mut self) {}

    fn reload_scripts(&mut self, _scripts: VarArray, _soft_reload: bool) {}

    fn reload_tool_script(&mut self, _script: Option<Gd<Script>>, _soft_reload: bool) {}

    fn get_recognized_extensions(&self) -> PackedStringArray {
        PackedStringArray::from(&[GString::from("rb")])
    }

    fn get_public_functions(&self) -> Array<AnyDictionary> {
        Array::new()
    }

    fn get_public_constants(&self) -> AnyDictionary {
        empty_dictionary()
    }

    fn get_public_annotations(&self) -> Array<AnyDictionary> {
        Array::new()
    }

    fn profiling_start(&mut self) {}

    fn profiling_stop(&mut self) {}

    fn profiling_set_save_native_calls(&mut self, _enable: bool) {}

    unsafe fn profiling_get_accumulated_data_rawptr(
        &mut self,
        _info_array: RawPtr<*mut ScriptLanguageExtensionProfilingInfo>,
        _info_max: i32,
    ) -> i32 {
        0
    }

    unsafe fn profiling_get_frame_data_rawptr(
        &mut self,
        _info_array: RawPtr<*mut ScriptLanguageExtensionProfilingInfo>,
        _info_max: i32,
    ) -> i32 {
        0
    }

    fn frame(&mut self) {}

    fn handles_global_class_type(&self, _type: GString) -> bool {
        false
    }

    fn get_global_class_name(&self, _path: GString) -> AnyDictionary {
        empty_dictionary()
    }
}
