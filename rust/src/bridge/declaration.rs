//! What a class body under Godot declares as it runs: its signals, its
//! exports and the headings they are shown under, taken for the realm to
//! publish once the file has run.

use beni::{
    Error, FromValue, Mrb, Object as _, RArray, RClass, ReprValue, Value, method, value::qnil,
};
use godot::builtin::{StringName, VarArray, Variant, VariantType};
use godot::classes::{ClassDb, Object, ProjectSettings, ResourceLoader, Script};
use godot::meta::ToGodot;
use godot::obj::{EngineEnum, Gd, Singleton};
use godot::register::info::PropertyHint;

use super::file_by_class_path;
use super::object::find_engine_ancestor;
use super::refusal::make_argument_error;
use super::value::{self, RubyCrossing};
use super::value_type::kind_by_name;
use crate::hint::{Hint, name_type};
use crate::realm;
use crate::snapshot::{Heading, Parameter, Property, Rpc, Signal};

/// Defines the methods Godot::Object's class bodies declare through.
pub(super) fn define(mrb: &Mrb, object: RClass) -> Result<(), Error> {
    object.define_singleton_method(mrb, c"__declare_signal__", method!(declare_signal, 2))?;
    object.define_singleton_method(mrb, c"__declare_export__", method!(declare_export, 4))?;
    object.define_singleton_method(mrb, c"__declare_heading__", method!(declare_heading, 3))?;
    object.define_singleton_method(mrb, c"__declare_rpc__", method!(declare_rpc, 3))?;
    Ok(())
}

// Godot::Object.__declare_signal__(name, parameters): takes the signal the
// class declares as its body runs, each parameter a name and the class it is
// declared with, for the realm to publish once the file has run.
fn declare_signal(
    mrb: &Mrb,
    class: RClass,
    name: String,
    parameters: RArray,
) -> Result<Value, Error> {
    let parameters = parameters
        .entries(mrb)
        .filter_map(RArray::from_value)
        .map(|pair| {
            let mut entries = pair.entries(mrb);
            let name = entries
                .next()
                .and_then(String::from_value)
                .unwrap_or_default();
            let class = entries.next().unwrap_or_else(|| qnil().as_value());
            read_parameter(mrb, name, class).map_err(|reason| make_argument_error(mrb, &reason))
        })
        .collect::<Result<_, _>>()?;
    realm::declare_signal(mrb, class, Signal { name, parameters })?;
    Ok(qnil().as_value())
}

// The parameter `name` of the class `class` names, typed as GDScript types a
// parameter of that type: a Ruby class as the engine type its values convert
// to, a class under Godot as its value type or engine class, and a node
// script's class as an object of the name the editor lists it by, or of the
// engine class it extends when it is listed by none.
fn read_parameter(mrb: &Mrb, name: String, class: Value) -> Result<Parameter, String> {
    let path = class
        .funcall(mrb, c"to_s", &[])
        .ok()
        .and_then(String::from_value)
        .unwrap_or_default();
    let builtin = match path.as_str() {
        "Godot::Variant" => Some(VariantType::NIL),
        "Godot::Bool" => Some(VariantType::BOOL),
        "Integer" => Some(VariantType::INT),
        "Float" => Some(VariantType::FLOAT),
        "String" => Some(VariantType::STRING),
        "Symbol" => Some(VariantType::STRING_NAME),
        "Array" => Some(VariantType::ARRAY),
        "Hash" => Some(VariantType::DICTIONARY),
        _ => None,
    };
    if let Some(kind) = builtin {
        return Ok(Parameter::new(name, kind));
    }
    if let Some(engine_name) = path.strip_prefix("Godot::") {
        if let Some(kind) = kind_by_name(engine_name) {
            return Ok(Parameter::new(name, kind));
        }
        if ClassDb::singleton().class_exists(engine_name) {
            return Ok(Parameter::new(name, VariantType::OBJECT).with_class(engine_name.to_owned()));
        }
        return Err(describe_unknown_type(&path));
    }
    let engine_class = RClass::from_value(class)
        .and_then(|class| find_engine_ancestor(mrb, class))
        .ok_or_else(|| describe_unknown_type(&path))?;
    let class_name = editor_name_by_class_path(mrb, &path).unwrap_or(engine_class);
    Ok(Parameter::new(name, VariantType::OBJECT).with_class(class_name))
}

// Godot::Object.__declare_export__(name, default, hint, written): takes the
// property the class exports as its body runs, its type read from the value
// it is declared with and its hint from the keyword naming it, read with
// the value written with that keyword, for the realm to publish once the
// file has run. A declaration GDScript would refuse is refused in
// GDScript's own words, so the two languages read the same when the same
// mistake is made.
fn declare_export(
    mrb: &Mrb,
    class: RClass,
    name: String,
    default: Value,
    hint: String,
    written: Value,
) -> Result<Value, Error> {
    let refuse = |reason: String| make_argument_error(mrb, &reason);
    if let Some(engine_class) = engine_class_by_member(mrb, class, &name) {
        let message =
            format!("Member \"{name}\" redefined (original in native class '{engine_class}')");
        return Err(make_argument_error(mrb, &message));
    }
    let data = super::data(mrb);
    let default = value::to_engine(mrb, data, default, 1).map_err(refuse)?;
    if hint == "typed" {
        let elements = RArray::from_value(written)
            .map(|classes| classes.entries(mrb).collect::<Vec<_>>())
            .unwrap_or_default();
        let (property, types) =
            declare_typed_property(mrb, name, &default, &elements).map_err(refuse)?;
        realm::declare_export(mrb, class, property)?;
        return Ok(RubyCrossing(&types.to_variant()).into_ruby(mrb, data));
    }
    let written = value::to_engine(mrb, data, written, 1).map_err(refuse)?;
    let property = match hint.as_str() {
        "type" => declare_class_property(mrb, name, &default, &written.to_string()),
        keyword => {
            let hint = Hint::from_keyword(keyword).map_err(refuse)?;
            let written = match hint {
                Hint::NodePath => name_node_types(mrb, &written),
                _ => written,
            };
            declare_value_property(name, &default, hint, &written)
        }
    }
    .map_err(refuse)?;
    realm::declare_export(mrb, class, property)?;
    Ok(qnil().as_value())
}

// The node types `written` names by their classes' paths, by the names the
// editor knows them by, as `@export_node_path` takes them; `true` names none.
fn name_node_types(mrb: &Mrb, written: &Variant) -> Variant {
    let Ok(paths) = written.try_to::<VarArray>() else {
        return written.clone();
    };
    paths
        .iter_shared()
        .map(|path| {
            let path = path.to_string();
            match path.strip_prefix("Godot::") {
                Some(engine_class) => engine_class.to_owned(),
                None => editor_name_by_class_path(mrb, &path).unwrap_or(path),
            }
            .to_variant()
        })
        .collect::<VarArray>()
        .to_variant()
}

// Godot::Object.__declare_rpc__(method, words, channel): takes the rpc the
// class declares for its method as its body runs, its mode, sync and
// transfer mode as `@rpc` words them, for the realm to publish once the file
// has run.
fn declare_rpc(
    mrb: &Mrb,
    class: RClass,
    method: String,
    words: RArray,
    channel: i64,
) -> Result<Value, Error> {
    let words: Vec<String> = words.entries(mrb).filter_map(String::from_value).collect();
    let [mode, sync, transfer_mode] = words.as_slice() else {
        return Err(make_argument_error(
            mrb,
            "an rpc takes a mode, a sync and a transfer mode",
        ));
    };
    let rpc = Rpc::read(method, mode, sync, transfer_mode, channel)
        .map_err(|reason| make_argument_error(mrb, &reason))?;
    realm::declare_rpc(mrb, class, rpc)?;
    Ok(qnil().as_value())
}

// Godot::Object.__declare_heading__(name, prefix, kind): takes the heading
// the class writes as its body runs, which the properties written after it
// are shown under, for the realm to publish once the file has run.
fn declare_heading(mrb: &Mrb, class: RClass, name: String, prefix: String, kind: String) -> Value {
    let heading = match kind.as_str() {
        // A category the body writes is headed by no file.
        "category" => Heading::Category {
            name,
            path: String::new(),
        },
        "subgroup" => Heading::Subgroup { name, prefix },
        _ => Heading::Group { name, prefix },
    };
    realm::declare_heading(mrb, class, heading);
    qnil().as_value()
}

// The property an export naming its type declares: an object of the class
// it names, which Godot fills in from the scene or the project's files. The
// value it is declared with is the class's own to hold, so a value of
// another type is refused as GDScript refuses a mismatched one.
fn declare_class_property(
    mrb: &Mrb,
    name: String,
    default: &Variant,
    class: &str,
) -> Result<Property, String> {
    let (hint, class_name) = hint_by_class(mrb, class)?;
    if let Some(given) = describe_mismatch(mrb, default, class) {
        return Err(format!(
            "Cannot assign a value of type {given} to variable \"{name}\" with specified type {class_name}."
        ));
    }
    Ok(Property::new(name, default).with_class(hint, class_name))
}

// What the value an export was declared with is, unless the class it names
// takes it: nothing at all, which is what Godot fills in, or an object of
// that class, a class descending from it included.
fn describe_mismatch(mrb: &Mrb, default: &Variant, class: &str) -> Option<String> {
    let kind = default.get_type();
    if kind == VariantType::NIL {
        return None;
    }
    let Ok(object) = default.try_to::<Gd<Object>>() else {
        return Some(name_type(kind));
    };
    (!is_of_class(mrb, &object, class)).then(|| resolve_class_name(&object))
}

// Whether `object` is of the class `class` names: the engine's class
// database answers for an engine class, and a node is of a Ruby class when
// its script is that class's file or inherits from it.
fn is_of_class(mrb: &Mrb, object: &Gd<Object>, class: &str) -> bool {
    if let Some(engine_class) = class.strip_prefix("Godot::") {
        let object_class = StringName::from(&object.get_class());
        return ClassDb::singleton().is_parent_class(&object_class, engine_class);
    }
    let Some(file) = file_by_class_path(mrb, class) else {
        return false;
    };
    collect_script_files(object).any(|path| path == file)
}

// The name `object`'s own class is known by: the one its node script is
// announced under, or the engine class it is of.
fn resolve_class_name(object: &Gd<Object>) -> String {
    collect_script_files(object)
        .next()
        .and_then(|path| editor_name_by_path(&path))
        .unwrap_or_else(|| object.get_class().to_string())
}

// The files `object`'s script is written in, nearest first: its own and the
// ones it inherits from, as Godot answers for them.
fn collect_script_files(object: &Gd<Object>) -> impl Iterator<Item = String> {
    let mut script = object.get_script().map(Gd::upcast::<Script>);
    std::iter::from_fn(move || {
        let written = script.take()?;
        script = written.get_base_script();
        Some(written.get_path().to_string())
    })
}

// The property an export naming no type declares, as its keyword says.
fn declare_value_property(
    name: String,
    default: &Variant,
    hint: Hint,
    written: &Variant,
) -> Result<Property, String> {
    hint.declare(name, default, written)
}

// The property an export naming its container's element types declares, as
// a GDScript variable typed `Array[T]` or `Dictionary[K, V]` is: its value
// is an array for one type and a dictionary for two, and the editor reads
// each type. What a container of those types is made with goes back to the
// class, which types the containers its objects hold.
fn declare_typed_property(
    mrb: &Mrb,
    name: String,
    default: &Variant,
    classes: &[Value],
) -> Result<(Property, VarArray), String> {
    let elements = classes
        .iter()
        .map(|class| read_element_type(mrb, *class))
        .collect::<Result<Vec<_>, _>>()?;
    let (container, typed_name) = match elements.as_slice() {
        [element] => (
            VariantType::ARRAY,
            format!("Array[{}]", element.type_name()),
        ),
        [key, value] => (
            VariantType::DICTIONARY,
            format!("Dictionary[{}, {}]", key.type_name(), value.type_name()),
        ),
        _ => return Err("\"type:\" needs one class in an array or one pair in a hash.".to_owned()),
    };
    if default.get_type() != container {
        return Err(format!(
            "Cannot assign a value of type \"{}\" as \"{typed_name}\".",
            name_type(default.get_type())
        ));
    }
    let hint_string = elements
        .iter()
        .map(ElementType::write_hint_string)
        .collect::<Vec<_>>()
        .join(";");
    let types = elements.iter().map(ElementType::describe).collect();
    let property = Property::new(name, default).with_hint(PropertyHint::TYPE_STRING, hint_string);
    Ok((property, types))
}

// What a typed container's element is: its type, the hint and name the
// editor reads an object of it by, and the engine class and script a
// container is typed with.
struct ElementType {
    kind: VariantType,
    hint: PropertyHint,
    name: String,
    engine_class: String,
    script: Option<Gd<Script>>,
}

impl ElementType {
    // The element's type as GDScript names it in a typed container.
    fn type_name(&self) -> String {
        match self.kind {
            VariantType::OBJECT => self.name.clone(),
            VariantType::NIL => "Variant".to_owned(),
            kind => name_type(kind),
        }
    }

    // The element as the editor reads a typed container's type.
    fn write_hint_string(&self) -> String {
        match self.kind {
            VariantType::OBJECT => format!(
                "{}/{}:{}",
                VariantType::OBJECT.ord(),
                self.hint.ord(),
                self.name
            ),
            kind => format!("{}:", kind.ord()),
        }
    }

    // What a container is typed with for the element: its type, its engine
    // class and its script.
    fn describe(&self) -> Variant {
        let script = self
            .script
            .as_ref()
            .map_or(Variant::nil(), ToGodot::to_variant);
        let mut described = VarArray::new();
        described.push(&self.kind.ord().to_variant());
        described.push(&StringName::from(&self.engine_class).to_variant());
        described.push(&script);
        described.to_variant()
    }
}

// The element type the class `class` names, as a parameter's type is read,
// an object being a resource or a node, as an export's is.
fn read_element_type(mrb: &Mrb, class: Value) -> Result<ElementType, String> {
    let path = class
        .funcall(mrb, c"to_s", &[])
        .ok()
        .and_then(String::from_value)
        .unwrap_or_default();
    let parameter = read_parameter(mrb, String::new(), class)?;
    if parameter.kind != VariantType::OBJECT {
        return Ok(ElementType {
            kind: parameter.kind,
            hint: PropertyHint::NONE,
            name: String::new(),
            engine_class: String::new(),
            script: None,
        });
    }
    let (engine_class, script) = match path.strip_prefix("Godot::") {
        Some(engine_class) => (engine_class.to_owned(), None),
        None => (
            RClass::from_value(class)
                .and_then(|class| find_engine_ancestor(mrb, class))
                .ok_or_else(|| describe_unknown_type(&path))?,
            file_by_class_path(mrb, &path)
                .and_then(|file| ResourceLoader::singleton().load(&file))
                .and_then(|resource| resource.try_cast::<Script>().ok()),
        ),
    };
    let class_db = ClassDb::singleton();
    let hint = if class_db.is_parent_class(&engine_class, "Resource") {
        PropertyHint::RESOURCE_TYPE
    } else if class_db.is_parent_class(&engine_class, "Node") {
        PropertyHint::NODE_TYPE
    } else {
        return Err("Export type can only be built-in, a resource, a node, or an enum.".to_owned());
    };
    Ok(ElementType {
        kind: VariantType::OBJECT,
        hint,
        name: parameter.class_name,
        engine_class,
        script,
    })
}

// The hint an exported object takes from the class it names, and the name
// the editor reads that class by: a resource is chosen among the project's
// files and a node among the scene's, while a class of neither kind is no
// type an export can take. A Ruby class is named by the announcement the
// editor lists it under, so one no announcement names is out of reach, and
// is a resource when its script extends one.
fn hint_by_class(mrb: &Mrb, class: &str) -> Result<(PropertyHint, String), String> {
    if let Some(engine_class) = class.strip_prefix("Godot::") {
        let class_db = ClassDb::singleton();
        if !class_db.class_exists(engine_class) {
            return Err(describe_unknown_type(class));
        }
        if class_db.is_parent_class(engine_class, "Resource") {
            return Ok((PropertyHint::RESOURCE_TYPE, engine_class.to_owned()));
        }
        if class_db.is_parent_class(engine_class, "Node") {
            return Ok((PropertyHint::NODE_TYPE, engine_class.to_owned()));
        }
        return Err("Export type can only be built-in, a resource, a node, or an enum.".to_owned());
    }
    let name = editor_name_by_class_path(mrb, class).ok_or_else(|| describe_unknown_type(class))?;
    let is_resource = file_by_class_path(mrb, class)
        .and_then(|file| ResourceLoader::singleton().load(&file))
        .and_then(|resource| resource.try_cast::<Script>().ok())
        .is_some_and(|script| {
            ClassDb::singleton().is_parent_class(&script.get_instance_base_type(), "Resource")
        });
    let hint = if is_resource {
        PropertyHint::RESOURCE_TYPE
    } else {
        PropertyHint::NODE_TYPE
    };
    Ok((hint, name))
}

// The name the editor lists the Ruby class `class` under, if a script of
// the project defines it and nothing else is announced by that name.
fn editor_name_by_class_path(mrb: &Mrb, class: &str) -> Option<String> {
    editor_name_by_path(&file_by_class_path(mrb, class)?)
}

// The name Godot lists the file at `path` under among its global classes,
// as GDScript finds a class an export names.
fn editor_name_by_path(path: &str) -> Option<String> {
    ProjectSettings::singleton()
        .get_global_class_list()
        .iter_shared()
        .find(|class| class.get_or_nil("path").to_string() == path)
        .map(|class| class.get_or_nil("class").to_string())
}

// GDScript's words for a type nothing in the project is known by.
fn describe_unknown_type(class: &str) -> String {
    format!("The class \"{class}\" was not found in the global scope.")
}

// The engine class `class` extends that has a member of that name, if one
// has: a signal, a property or an integer constant, which is what GDScript
// refuses a member for redefining.
fn engine_class_by_member(mrb: &Mrb, class: RClass, name: &str) -> Option<String> {
    let engine_class = find_engine_ancestor(mrb, class)?;
    let mut class_db = ClassDb::singleton();
    let has_member = class_db.class_has_signal(engine_class.as_str(), name)
        || class_db.class_has_integer_constant(engine_class.as_str(), name)
        || !class_db
            .class_get_property_setter(engine_class.as_str(), name)
            .is_empty()
        || !class_db
            .class_get_property_getter(engine_class.as_str(), name)
            .is_empty();
    has_member.then_some(engine_class)
}
