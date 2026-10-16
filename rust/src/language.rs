use std::ffi::c_void;
use std::sync::{Mutex, PoisonError};

use godot::classes::native::ScriptLanguageExtensionProfilingInfo;
use godot::classes::script_language_extension::{CodeCompletionKind, CodeCompletionLocation};
use godot::classes::{
    ClassDb, EditorInterface, Engine, IScriptLanguageExtension, Object, ProjectSettings,
    ResourceLoader, Script, ScriptLanguageExtension,
};
use godot::global::Error;
use godot::meta::conv::RawPtr;
use godot::prelude::*;
use godot::register::info::PropertyUsageFlags;

use crate::announcement::{self, Omission, SharedNames};
use crate::compiler::CompileError;
use crate::completion::{self, Candidate};
use crate::game::{self, GameFiles};
use crate::realm::{Files, Location};
use crate::script::RubyScript;
use crate::template;
use crate::validation::{self, Warning};
use crate::{bridge, log, settings, snapshot, warn};

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
static SHARED_NAMES: Mutex<SharedNames> = Mutex::new(SharedNames::new());

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

    // The file's name comes without its directory; the class is named after
    // the file, and extends the base as Ruby writes it.
    fn make_template(
        &self,
        template: GString,
        class_name: GString,
        base_class_name: GString,
    ) -> Option<Gd<Script>> {
        let base = resolve_base(&base_class_name.to_string());
        let superclass = template::write_superclass(&base, &GameFiles.roots());
        let source = template::write_source(
            &template.to_string(),
            &class_name.to_string(),
            &superclass,
            &indentation(),
        );
        Some(RubyScript::from_template(GString::from(&source)).upcast())
    }

    fn get_built_in_templates(&self, object: StringName) -> Array<AnyDictionary> {
        template::list_built_ins(&object.to_string())
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
        let checked = validation::validate(
            game::listing(),
            &test_directories,
            &settings::template_directory(),
            &validation::EngineClasses {
                is_node: &game::is_node_class,
                method_declarer: &bridge::find_method_declarer,
            },
            &path.to_string(),
            &script.to_string(),
        );
        let errors: VarArray = checked
            .error
            .iter()
            .map(|error| describe_error(error, &path).to_variant())
            .collect();
        let warnings: VarArray = checked
            .warnings
            .iter()
            .map(|warning| describe_warning(warning).to_variant())
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
        code: GString,
        path: GString,
        _owner: Option<Gd<Object>>,
    ) -> AnyDictionary {
        let code = code.to_string();
        let script = completion::read_script_names(
            &GameFiles,
            &snapshot::latest(),
            &path.to_string(),
            &code,
        );
        let engine = completion::EngineNames {
            classes: &collect_class_names,
            members: &collect_engine_members,
        };
        let options: VarArray = completion::collect_candidates(&code, &script, &engine)
            .iter()
            .map(|candidate| describe_option(candidate).to_variant())
            .collect();
        vdict! { "result" => Error::OK, "force" => false, "call_hint" => "", "options" => &options }
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
        log::current_backtrace()
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
        let mut shared_names = SHARED_NAMES.lock().unwrap_or_else(PoisonError::into_inner);
        if !matches!(announced, Err(Omission::SharedName { .. })) {
            shared_names.forget(&file);
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
                if shared_names.note(&name, &others) {
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
fn describe_error(error: &CompileError, path: &GString) -> VarDictionary {
    vdict! {
        "path" => path,
        "line" => error.line,
        "column" => error.column,
        "message" => error.message.as_str(),
    }
}

// A name the script editor offers, as it takes each: every key is read.
fn describe_option(candidate: &Candidate) -> VarDictionary {
    let kind = match candidate.kind {
        completion::Kind::Class => CodeCompletionKind::CLASS,
        completion::Kind::Method { .. } => CodeCompletionKind::FUNCTION,
        completion::Kind::Property => CodeCompletionKind::MEMBER,
        completion::Kind::Constant => CodeCompletionKind::CONSTANT,
    };
    let location = match candidate.origin {
        completion::Origin::Script => CodeCompletionLocation::LOCAL,
        completion::Origin::Project => CodeCompletionLocation::OTHER_USER_CODE,
        completion::Origin::Engine => CodeCompletionLocation::OTHER,
    };
    vdict! {
        "kind" => kind,
        "display" => candidate.display(),
        "insert_text" => candidate.insert_text(),
        "font_color" => Color::WHITE,
        "icon" => &Variant::nil(),
        "default_value" => &Variant::nil(),
        "location" => location,
    }
}

// The methods, properties and integer constants of the engine class
// `class`, its ancestors' included, as Ruby reaches them on its objects; a
// property list's groups and indexed properties name nothing Ruby calls.
fn collect_engine_members(class: &str) -> Vec<Candidate> {
    let class_db = ClassDb::singleton();
    let member = |kind, name: String| Candidate {
        kind,
        origin: completion::Origin::Engine,
        name,
    };
    let methods = class_db.class_get_method_list(class);
    let methods = methods.iter_shared().map(|method| {
        let has_arguments = method
            .get("args")
            .and_then(|arguments| arguments.try_to::<AnyArray>().ok())
            .is_some_and(|arguments| !arguments.is_empty());
        member(
            completion::Kind::Method {
                has_arguments: Some(has_arguments),
            },
            method
                .get("name")
                .map(|name| name.to_string())
                .unwrap_or_default(),
        )
    });
    let group_usage =
        PropertyUsageFlags::CATEGORY | PropertyUsageFlags::GROUP | PropertyUsageFlags::SUBGROUP;
    let properties = class_db.class_get_property_list(class);
    let properties = properties
        .iter_shared()
        .filter(|property| {
            let usage = property
                .get("usage")
                .and_then(|usage| usage.try_to::<i64>().ok())
                .unwrap_or_default();
            usage.cast_unsigned() & group_usage.ord() == 0
        })
        .filter_map(|property| property.get("name").map(|name| name.to_string()))
        .filter(|name| !name.contains('/'))
        .map(|name| member(completion::Kind::Property, name));
    let constants = class_db.class_get_integer_constant_list(class);
    let constants = constants
        .as_slice()
        .iter()
        .map(|name| member(completion::Kind::Constant, name.to_string()));
    methods.chain(properties).chain(constants).collect()
}

// The names of the classes under `Godot`: every class the engine registers
// and its value types, as Ruby names them there.
fn collect_class_names() -> Vec<String> {
    ClassDb::singleton()
        .get_class_list()
        .as_slice()
        .iter()
        .map(GString::to_string)
        .chain(bridge::collect_value_type_names())
        .collect()
}

// A warning as the script editor takes it, listed under its kind.
fn describe_warning(warning: &Warning) -> VarDictionary {
    vdict! {
        "start_line" => warning.line,
        "end_line" => warning.line,
        "code" => warning.kind as i32,
        "string_code" => warning.kind.code(),
        "message" => warning.message.as_str(),
    }
}

// Warns that the node scripts in `files`, in order, share `name`, at the
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

// Where Godot's ScriptLanguage::TemplateLocation places a language's own.
const TEMPLATE_BUILT_IN: i64 = 0;

// A base the dialog names by a script's global class, as the quoted path of
// that script, which is how the dialog names one it only knows by path.
fn resolve_base(base: &str) -> String {
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
pub fn indentation() -> String {
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
