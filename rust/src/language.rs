use std::ffi::c_void;
use std::sync::{Mutex, PoisonError};

use godot::classes::native::ScriptLanguageExtensionProfilingInfo;
use godot::classes::{
    Engine, IScriptLanguageExtension, Object, ResourceLoader, Script, ScriptLanguageExtension,
};
use godot::global::Error;
use godot::meta::conv::RawPtr;
use godot::prelude::*;

use crate::announcement::{self, NameClashes, Omission};
use crate::game::GameFiles;
use crate::realm::{Files, Location};
use crate::script::RubyScript;
use crate::{log, warn};

#[cfg(feature = "editor")]
mod editor;

// Only the editor asks these, so a library built without it answers each as
// a language with nothing to offer.
#[cfg(not(feature = "editor"))]
mod editor {
    use godot::classes::Script;
    use godot::prelude::*;

    pub fn make_template(_: GString, _: GString, _: GString) -> Option<Gd<Script>> {
        None
    }

    pub fn list_built_in_templates(_: StringName) -> Array<AnyDictionary> {
        Array::new()
    }

    pub fn validate(_: GString, _: GString) -> AnyDictionary {
        super::empty_dictionary()
    }

    pub fn complete_code(_: GString, _: GString) -> AnyDictionary {
        super::empty_dictionary()
    }

    pub fn lookup_code(_: GString, _: GString, _: GString) -> AnyDictionary {
        super::empty_dictionary()
    }
}

#[cfg(feature = "editor")]
pub use editor::indentation;

/// The Ruby script language, registered with the engine for `.rb` files.
///
/// Every virtual answers from constants or the calling thread's own state:
/// Godot asks the language for a backtrace while it prints an error, from
/// whichever thread raised it, so nothing here may wait on anything.
#[derive(GodotClass)]
#[class(base = ScriptLanguageExtension, init, tool)]
pub struct RubyLanguage {
    base: Base<ScriptLanguageExtension>,
}

/// The language's name, by which the editor matches it with a highlighter.
pub const NAME: &str = "Ruby";

static LANGUAGE: Mutex<Option<InstanceId>> = Mutex::new(None);
static NAME_CLASHES: Mutex<NameClashes> = Mutex::new(NameClashes::new());

pub fn register() {
    let language = RubyLanguage::new_alloc();
    Engine::singleton().register_script_language(&language);
    *LANGUAGE.lock().unwrap() = Some(language.instance_id());
}

pub fn unregister() {
    if let Some(id) = LANGUAGE.lock().unwrap().take() {
        let language = Gd::<RubyLanguage>::from_instance_id(id);
        Engine::singleton().unregister_script_language(&language);
        language.free();
    }
}

/// The registered language, which every script reports as its own.
pub fn language() -> Option<Gd<RubyLanguage>> {
    LANGUAGE.lock().unwrap().map(Gd::from_instance_id)
}

// The stack this thread is reporting, nearest first. With the debugger, an
// error the engine reports inside a Ruby call reports the frames Ruby runs.
fn reported_stack() -> Vec<Location> {
    let backtrace = log::current_backtrace();
    #[cfg(feature = "debugger")]
    if backtrace.is_empty() {
        return crate::realm::collect_running_backtrace();
    }
    backtrace
}

// The frame `level` of the stack this thread is reporting, nearest first.
fn reported_frame(level: i32) -> Option<Location> {
    let level = usize::try_from(level).ok()?;
    reported_stack().into_iter().nth(level)
}

fn empty_dictionary() -> AnyDictionary {
    VarDictionary::new().upcast_any_dictionary()
}

#[godot_api]
impl IScriptLanguageExtension for RubyLanguage {
    fn get_name(&self) -> GString {
        NAME.into()
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
        KEYWORDS.iter().map(|word| GString::from(*word)).collect()
    }

    fn is_control_flow_keyword(&self, keyword: GString) -> bool {
        is_control_flow(&keyword.to_string())
    }

    // The editor comments lines out with it, and reads a project template's
    // `# meta-` lines by it.
    fn get_comment_delimiters(&self) -> PackedStringArray {
        PackedStringArray::from(&[GString::from("#")])
    }

    // A start and an end apart from each other by a space. The editor colours
    // what lies between, honours a backslash escape there, and closes the
    // quote as it is typed.
    fn get_string_delimiters(&self) -> PackedStringArray {
        PackedStringArray::from(&[GString::from("\" \""), GString::from("' '")])
    }

    fn make_template(
        &self,
        template: GString,
        class_name: GString,
        base_class_name: GString,
    ) -> Option<Gd<Script>> {
        editor::make_template(template, class_name, base_class_name)
    }

    fn get_built_in_templates(&self, object: StringName) -> Array<AnyDictionary> {
        editor::list_built_in_templates(object)
    }

    fn is_using_templates(&mut self) -> bool {
        true
    }

    fn validate(
        &self,
        script: GString,
        path: GString,
        _validate_functions: bool,
        _validate_errors: bool,
        _validate_warnings: bool,
        _validate_safe_lines: bool,
    ) -> AnyDictionary {
        editor::validate(script, path)
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
        true
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

    fn complete_code(
        &self,
        code: GString,
        path: GString,
        _owner: Option<Gd<Object>>,
    ) -> AnyDictionary {
        editor::complete_code(code, path)
    }

    fn lookup_code(
        &self,
        code: GString,
        symbol: GString,
        path: GString,
        _owner: Option<Gd<Object>>,
    ) -> AnyDictionary {
        editor::lookup_code(code, symbol, path)
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
        #[cfg(feature = "debugger")]
        return crate::debugger::stop_reason();
        #[cfg(not(feature = "debugger"))]
        GString::new()
    }

    // A virtual never runs Ruby, so each stack level is a frame of what this
    // thread is reporting: a debug break, an exception being written, or an
    // error Ruby pushes.
    fn debug_get_stack_level_count(&self) -> i32 {
        reported_stack().len() as i32
    }

    fn debug_get_stack_level_line(&self, level: i32) -> i32 {
        reported_frame(level).map_or(-1, |frame| frame.line as i32)
    }

    fn debug_get_stack_level_function(&self, level: i32) -> GString {
        reported_frame(level).map_or_else(GString::new, |frame| frame.function.as_str().into())
    }

    fn debug_get_stack_level_source(&self, level: i32) -> GString {
        reported_frame(level).map_or_else(GString::new, |frame| frame.file.as_str().into())
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
        reported_stack()
            .iter()
            .map(|frame| {
                vdict! {
                    "file" => frame.file.as_str(),
                    "func" => frame.function.as_str(),
                    "line" => frame.line,
                }
                .upcast_any_dictionary()
            })
            .collect()
    }

    // What the debugger asks of a running game when the editor cannot say
    // which scripts changed: every Ruby script Godot holds reads its file
    // again, as a GDScript does.
    fn reload_all_scripts(&mut self) {
        let loader = ResourceLoader::singleton();
        for path in GameFiles.paths() {
            if let Some(script) = loader
                .get_cached_ref(&path)
                .and_then(|resource| resource.try_cast::<RubyScript>().ok())
            {
                RubyScript::reload_from_disk(script);
            }
        }
    }

    // What the debugger asks of a running game when the editor saves
    // scripts: each Ruby one reads its file again, as a GDScript does. The
    // objects a file made keep their state however soft the reload.
    fn reload_scripts(&mut self, scripts: VarArray, _soft_reload: bool) {
        for script in scripts.iter_shared() {
            if let Ok(script) = script.try_to::<Gd<RubyScript>>() {
                RubyScript::reload_from_disk(script);
            }
        }
    }

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

    fn handles_global_class_type(&self, type_: GString) -> bool {
        type_ == RubyScript::class_id().to_gstring()
    }

    // The editor asks this of every script file as it scans the project, and
    // lists the file as a class by the name answered.
    fn get_global_class_name(&self, path: GString) -> AnyDictionary {
        let file = path.to_string();
        let announced = announcement::announcement_by_path(&file);
        let mut clashes = NAME_CLASHES.lock().unwrap_or_else(PoisonError::into_inner);
        if !matches!(announced, Err(Omission::SharedName { .. })) {
            clashes.forget(&file);
        }
        match announced {
            Ok(announcement) => vdict! {
                "name" => announcement.name,
                "base_type" => announcement.base,
                "icon_path" => announcement.icon.unwrap_or_default(),
                "is_abstract" => announcement.is_abstract,
                "is_tool" => announcement.is_tool,
            }
            .upcast_any_dictionary(),
            Err(Omission::SharedName { name, mut others }) => {
                others.push(file);
                others.sort();
                if clashes.note(&name, &others) {
                    warn_of_shared_name(name, others);
                }
                empty_dictionary()
            }
            Err(_) => empty_dictionary(),
        }
    }
}

// Warns that the scripts in `files`, in order, share `name`, at the
// first of them. Written once the language is no longer in a call: Godot asks
// the language for its stack as it prints a warning, which cannot happen
// while it answers.
fn warn_of_shared_name(name: String, files: Vec<String>) {
    Callable::from_sync_fn("warn_of_shared_name", move |_| {
        let at = Location::new(files[0].clone(), 1);
        let warning = announcement::describe_shared_name(&files[0], &name, &files[1..]);
        warn!(at: &at, "{warning}");
    })
    .call_deferred(&[]);
}

/// Ruby's keywords, as the parser spells them.
pub const KEYWORDS: &[&str] = &[
    "__ENCODING__",
    "__FILE__",
    "__LINE__",
    "BEGIN",
    "END",
    "alias",
    "and",
    "begin",
    "break",
    "case",
    "class",
    "def",
    "defined?",
    "do",
    "else",
    "elsif",
    "end",
    "ensure",
    "false",
    "for",
    "if",
    "in",
    "module",
    "next",
    "nil",
    "not",
    "or",
    "redo",
    "rescue",
    "retry",
    "return",
    "self",
    "super",
    "then",
    "true",
    "undef",
    "unless",
    "until",
    "when",
    "while",
    "yield",
];

// The keywords that branch, loop or leave a block, which the editor colours
// apart from the rest.
const CONTROL_FLOW: &[&str] = &[
    "begin", "break", "case", "else", "elsif", "ensure", "for", "if", "in", "next", "redo",
    "rescue", "retry", "return", "then", "unless", "until", "when", "while", "yield",
];

pub fn is_control_flow(word: &str) -> bool {
    CONTROL_FLOW.contains(&word)
}

#[cfg(test)]
mod tests {
    use super::{CONTROL_FLOW, KEYWORDS};

    // The editor colours a control-flow keyword only when it is reserved.
    // @behavior RU-001
    #[test]
    fn a_control_flow_keyword_is_a_reserved_word() {
        let unreserved: Vec<_> = CONTROL_FLOW
            .iter()
            .filter(|word| !KEYWORDS.contains(word))
            .collect();

        assert!(unreserved.is_empty(), "not reserved: {unreserved:?}");
    }
}
