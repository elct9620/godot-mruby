use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::{Mutex, PoisonError};

use godot::classes::native::ScriptLanguageExtensionProfilingInfo;
use godot::classes::{
    ClassDb, EditorInterface, Engine, IScriptLanguageExtension, Object, ProjectSettings,
    ResourceLoader, Script, ScriptLanguageExtension,
};
use godot::global::Error;
use godot::meta::conv::RawPtr;
use godot::prelude::*;

use crate::announcement::{self, Clashes, Omission, Project};
use crate::compiler::CompileError;
use crate::game::{FilesOnDisk, GameFiles};
use crate::realm::{Files, Location};
use crate::script::RubyScript;
use crate::template;
use crate::validation::{self, Warning};
use crate::{bridge, settings, warn};

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

static LANGUAGE: Mutex<Option<InstanceId>> = Mutex::new(None);
static CLASHES: Mutex<Clashes> = Mutex::new(Clashes::new());

thread_local! {
    // What the language answers as this thread's stack. It holds no Godot
    // value, since a thread's locals outlive the engine.
    static STACK: RefCell<Vec<Location>> = const { RefCell::new(Vec::new()) };
}

/// Runs `write` while the language answers `backtrace` as this thread's
/// stack, so what it writes to Godot's log carries the frames.
pub fn write_with_stack(backtrace: &[Location], write: impl FnOnce()) {
    let answered = STACK.replace(backtrace.to_vec());
    write();
    STACK.set(answered);
}

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

    // The file's name comes without its directory; the class is named after
    // the file, and extends the base as Ruby writes it.
    fn make_template(
        &self,
        template: GString,
        class_name: GString,
        base_class_name: GString,
    ) -> Option<Gd<Script>> {
        let base = script_path_of(&base_class_name.to_string());
        let superclass = template::superclass(&base, &GameFiles.roots());
        let source = template::source(
            &template.to_string(),
            &class_name.to_string(),
            &superclass,
            &indentation(),
        );
        Some(RubyScript::from_source("", GString::from(&source)).upcast())
    }

    fn get_built_in_templates(&self, object: StringName) -> Array<AnyDictionary> {
        template::built_ins(&object.to_string())
            .map(|(id, template)| {
                vdict! {
                    "inherit" => template.inherit,
                    "name" => template.name,
                    "description" => template.description,
                    "content" => template.content,
                    "id" => id as i64,
                    "origin" => TEMPLATE_BUILT_IN,
                }
                .upcast_any_dictionary()
            })
            .collect()
    }

    fn is_using_templates(&mut self) -> bool {
        true
    }

    // The editor asks as the source is typed. It is compiled and never run,
    // and what the compiler says, with what the other files make of the
    // file's name, comes back as data: the language prints nothing while it
    // answers.
    fn validate(
        &self,
        script: GString,
        path: GString,
        _validate_functions: bool,
        _validate_errors: bool,
        _validate_warnings: bool,
        _validate_safe_lines: bool,
    ) -> AnyDictionary {
        let test_directories = settings::test_directories();
        let checked = validation::validation(
            &FilesOnDisk,
            &test_directories,
            &bridge::is_node_class,
            &path.to_string(),
            &script.to_string(),
        );
        let errors: VarArray = checked
            .error
            .iter()
            .map(|error| error_info(error, &path).to_variant())
            .collect();
        let warnings: VarArray = checked
            .warnings
            .iter()
            .map(|warning| warning_info(warning).to_variant())
            .collect();
        vdict! {
            "valid" => checked.error.is_none(),
            "errors" => &errors,
            "warnings" => &warnings,
        }
        .upcast_any_dictionary()
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

    // mruby 4.0 has no public API answering the stack Ruby is running, so
    // only an exception being written has frames to answer.
    fn debug_get_current_stack_info(&mut self) -> Array<AnyDictionary> {
        STACK.with_borrow(|stack| {
            stack
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
        })
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
        let test_directories = settings::test_directories();
        let project = Project::new(&FilesOnDisk, &test_directories, &bridge::is_node_class);
        let file = path.to_string();
        let announced = project.announcement(&file);
        let mut clashes = CLASHES.lock().unwrap_or_else(PoisonError::into_inner);
        if !matches!(announced, Err(Omission::SharedName { .. })) {
            clashes.forget(&file);
        }
        match announced {
            Ok(announcement) => vdict! {
                "name" => announcement.name,
                "base_type" => announcement.base,
                "icon_path" => announcement
                    .icon
                    .map(|icon| icon_path(&path, &icon))
                    .unwrap_or_default()
                    .to_string(),
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

// An error as the script editor takes it: at the script's own path, or it is
// shown as another file's.
fn error_info(error: &CompileError, path: &GString) -> VarDictionary {
    vdict! {
        "path" => path,
        "line" => error.line,
        "column" => error.column,
        "message" => error.message.as_str(),
    }
}

// A warning as the script editor takes it, listed under its kind.
fn warning_info(warning: &Warning) -> VarDictionary {
    vdict! {
        "start_line" => warning.line,
        "end_line" => warning.line,
        "code" => warning.kind as i32,
        "string_code" => warning.kind.code(),
        "message" => warning.message.as_str(),
    }
}

// An icon's path from the project's root: one written relative to the file's
// directory is taken from there, as GDScript's `@icon` is.
fn icon_path(script: &GString, icon: &str) -> GString {
    let icon = GString::from(icon);
    if icon.is_relative_path() {
        script.get_base_dir().path_join(&icon).simplify_path()
    } else {
        icon.simplify_path()
    }
}

// Warns that the node scripts in `files`, in order, share `name`, at the
// first of them. Written once the language is no longer in a call: Godot asks
// the language for its stack as it prints a warning, which cannot happen
// while it answers.
fn warn_of_shared_name(name: String, files: Vec<String>) {
    Callable::from_sync_fn("warn_of_shared_name", move |_| {
        let at = Location {
            file: files[0].clone(),
            line: 1,
            function: String::new(),
        };
        let warning = announcement::shared_name_warning(&files[0], &name, &files[1..]);
        warn!(at: &at, "{warning}");
    })
    .call_deferred(&[]);
}

// Where Godot's ScriptLanguage::TemplateLocation places a language's own.
const TEMPLATE_BUILT_IN: i64 = 0;

// A base the dialog names by a script's global class, as the quoted path of
// that script, which is how the dialog names one it only knows by path.
fn script_path_of(base: &str) -> String {
    if ClassDb::singleton().class_exists(base) {
        return base.to_owned();
    }
    ProjectSettings::singleton()
        .get_global_class_list()
        .iter_shared()
        .find(|class| class.get_or_nil("class").to_string() == base)
        .map_or_else(
            || base.to_owned(),
            |class| format!("\"{}\"", class.get_or_nil("path")),
        )
}

// One level of indentation as the editor writes it, as GDScript's templates
// take it: a tab, or the editor's number of spaces.
fn indentation() -> String {
    let settings = EditorInterface::singleton().get_editor_settings();
    let setting = |name: &str| {
        settings
            .as_ref()
            .map(|settings| settings.get_setting(name))
            .unwrap_or_default()
    };
    if setting("text_editor/behavior/indent/type").booleanize() {
        let size = setting("text_editor/behavior/indent/size").try_to::<u8>();
        " ".repeat(size.map_or(4, usize::from))
    } else {
        "\t".to_owned()
    }
}
