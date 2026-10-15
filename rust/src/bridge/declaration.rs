//! What a class body under Godot declares as it runs: its signals, its
//! exports and the headings they are shown under, taken for the realm to
//! publish once the file has run.

use beni::{
    Error, FromValue, Mrb, Object as _, RArray, RClass, ReprValue, Value, method, value::qnil,
};
use godot::builtin::{StringName, Variant, VariantType};
use godot::classes::{ClassDb, Object, ProjectSettings, Script};
use godot::obj::{Gd, Singleton};
use godot::register::info::PropertyHint;

use super::file_by_class_path;
use super::object::find_engine_ancestor;
use super::refusal::make_argument_error;
use super::value;
use super::value_type::kind_by_name;
use crate::hint::{Hint, name_type};
use crate::realm;
use crate::snapshot::{Heading, Parameter, Property, Signal};

/// Defines the methods Godot::Object's class bodies declare through.
pub(super) fn define(mrb: &Mrb, object: RClass) -> Result<(), Error> {
    object.define_singleton_method(mrb, c"__declare_signal__", method!(declare_signal, 2))?;
    object.define_singleton_method(mrb, c"__declare_export__", method!(declare_export, 4))?;
    object.define_singleton_method(mrb, c"__declare_heading__", method!(declare_heading, 3))?;
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
    let class_name = find_editor_name(mrb, &path).unwrap_or(engine_class);
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
    let keyword = match hint.as_str() {
        "type" => None,
        keyword => {
            Some(Hint::from_keyword(keyword).map_err(|reason| make_argument_error(mrb, &reason))?)
        }
    };
    let data = super::data(mrb);
    let default = value::to_engine(mrb, data, default, 1)
        .map_err(|reason| make_argument_error(mrb, &reason))?;
    let written = value::to_engine(mrb, data, written, 1)
        .map_err(|reason| make_argument_error(mrb, &reason))?;
    if let Some(engine_class) = find_engine_member(mrb, class, &name) {
        let message =
            format!("Member \"{name}\" redefined (original in native class '{engine_class}')");
        return Err(make_argument_error(mrb, &message));
    }
    let property = match keyword {
        None => declare_class_property(mrb, name, &default, &written.to_string()),
        Some(hint) => declare_value_property(name, &default, hint, &written),
    }
    .map_err(|reason| make_argument_error(mrb, &reason))?;
    realm::declare_export(mrb, class, property)?;
    Ok(qnil().as_value())
}

// Godot::Object.__declare_heading__(name, prefix, kind): takes the heading
// the class writes as its body runs, which the properties written after it
// are shown under, for the realm to publish once the file has run.
fn declare_heading(mrb: &Mrb, _class: RClass, name: String, prefix: String, kind: String) -> Value {
    let heading = match kind.as_str() {
        // A category the body writes is headed by no file.
        "category" => Heading::Category {
            name,
            path: String::new(),
        },
        "subgroup" => Heading::Subgroup { name, prefix },
        _ => Heading::Group { name, prefix },
    };
    realm::declare_heading(mrb, heading);
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

// The property an export naming no type declares: its type is the declared
// value's, so a value naming none is refused, and the keyword naming a hint
// tells the editor how to show it.
fn declare_value_property(
    name: String,
    default: &Variant,
    hint: Hint,
    written: &Variant,
) -> Result<Property, String> {
    if default.get_type() == VariantType::NIL {
        return Err(
            "Cannot use \"export\" because the type of the initialized value can't be inferred."
                .to_owned(),
        );
    }
    let property_hint = hint.choose_property_hint(default.get_type())?;
    // The gem refuses a value of another shape where it is written.
    let hint_string = hint.write_hint_string(written).unwrap_or_default();
    Ok(Property::new(name, default).with_hint(property_hint, hint_string))
}

// The hint an exported object takes from the class it names, and the name
// the editor reads that class by: a resource is chosen among the project's
// files and a node among the scene's, while a class of neither kind is no
// type an export can take. A Ruby class is named by the announcement the
// editor lists it under, so one no announcement names is out of reach.
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
    find_editor_name(mrb, class)
        .map(|name| (PropertyHint::NODE_TYPE, name))
        .ok_or_else(|| describe_unknown_type(class))
}

// The name the editor lists the Ruby class `class` under, if a node script
// of the project defines it and nothing else is announced by that name.
fn find_editor_name(mrb: &Mrb, class: &str) -> Option<String> {
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
fn find_engine_member(mrb: &Mrb, class: RClass, name: &str) -> Option<String> {
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
