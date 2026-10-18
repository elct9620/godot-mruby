//! What the language answers only the editor: templates, validation,
//! completion and lookup.

use godot::classes::script_language_extension::{
    CodeCompletionKind, CodeCompletionLocation, LookupResultType,
};
use godot::classes::{ClassDb, EditorInterface, ProjectSettings, ResourceLoader, Script};
use godot::global::Error;
use godot::prelude::*;
use godot::register::info::PropertyUsageFlags;

use crate::compiler::CompileError;
use crate::completion::{self, Candidate};
use crate::game::{self, GameFiles};
use crate::lookup::{self, Member, Target};
use crate::realm::Files;
use crate::script::RubyScript;
use crate::template;
use crate::validation::{self, Warning};
use crate::{bridge, settings, snapshot};

// The file's name comes without its directory; the class is named after
// the file, and extends the base as Ruby writes it.
pub fn make_template(
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

pub fn list_built_in_templates(object: StringName) -> Array<AnyDictionary> {
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

// The editor asks as the source is typed. It is compiled and never run,
// and what the compiler says, with what the other files make of the
// file's name, comes back as data: the language prints nothing while it
// answers.
pub fn validate(script: GString, path: GString) -> AnyDictionary {
    let test_directories = settings::test_directories();
    let checked = validation::validate(
        game::listing(),
        &test_directories,
        &settings::template_directory(),
        &validation::EngineClasses {
            is_engine_class: &game::is_engine_class,
            method_declarer: &bridge::find_method_declarer,
            is_string_name_member: &bridge::is_string_name_member,
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

// Godot reads these keys unconditionally and logs an error when one is missing.
pub fn complete_code(code: GString, path: GString) -> AnyDictionary {
    let code = code.to_string();
    let script = completion::read_script_names(
        game::listing(),
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
pub fn lookup_code(code: GString, symbol: GString, path: GString) -> AnyDictionary {
    let path = path.to_string();
    let engine = completion::EngineNames {
        classes: &collect_class_names,
        members: &collect_engine_members,
    };
    let target = lookup::find_target(
        game::listing(),
        &snapshot::latest(),
        &path,
        &code.to_string(),
        &symbol.to_string(),
        &engine,
    );
    match target {
        Some(target) => describe_target(&target, &path),
        None => vdict! { "result" => Error::ERR_UNAVAILABLE, "type" => LookupResultType::SCRIPT_LOCATION },
    }
    .upcast_any_dictionary()
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

// Where the script editor goes for a name, as it takes it: a line of
// another file opens that file's script, and one of the file at `path` stays
// in its editor.
fn describe_target(target: &Target, path: &str) -> VarDictionary {
    match target {
        Target::Line { path: file, line } => {
            let script = (file != path)
                .then(|| ResourceLoader::singleton().load(file.as_str()))
                .flatten();
            vdict! {
                "result" => Error::OK,
                "type" => LookupResultType::SCRIPT_LOCATION,
                "location" => i64::from(*line),
                "script" => &script.to_variant(),
            }
        }
        Target::EngineClass { class } => vdict! {
            "result" => Error::OK,
            "type" => LookupResultType::CLASS,
            "class_name" => class.as_str(),
        },
        Target::EngineMember {
            class,
            member,
            name,
        } => {
            let kind = match member {
                Member::Method => LookupResultType::CLASS_METHOD,
                Member::Property => LookupResultType::CLASS_PROPERTY,
                Member::Constant => LookupResultType::CLASS_CONSTANT,
            };
            vdict! {
                "result" => Error::OK,
                "type" => kind,
                "class_name" => class.as_str(),
                "class_member" => name.as_str(),
            }
        }
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

// Where Godot's ScriptLanguage::TemplateLocation places a language's own.
const TEMPLATE_BUILT_IN: i64 = 0;

// A base the dialog names by a script's global class, as the quoted path of
// that script, which is how the dialog names one it only knows by path.
fn resolve_base(base: &str) -> String {
    if game::is_engine_class(base) {
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
