//! The engine's objects as Ruby holds them: an object of a class under
//! `Godot` carries the engine object it stands for, and Ruby reaches its
//! methods by their names.

use beni::{
    Array, DataType, Error, ExceptionClass, FromValue, IntoValue, Module, Mrb, Object as _, RClass,
    RModule, ReprValue, Symbol, TryConvert, TypedData, Value, method,
};
use godot::builtin::StringName;
use godot::builtin::{Variant, VariantType};
use godot::classes::{ClassDb, Engine, Object, ResourceLoader, Script};
use godot::global::type_string;
use godot::meta::ToGodot;
use godot::meta::error::CallError;
use godot::obj::{EngineEnum, Gd, InstanceId, Singleton};
use godot::register::info::PropertyHint;

use super::value::{self, ToRuby};
use crate::announcement::Project;
use crate::game::FilesOnDisk;
use crate::realm::{self, Key};
use crate::settings;
use crate::snapshot::{Heading, Property, Signal};

/// The engine object a Ruby object of an engine class stands for. Holding
/// it keeps a reference-counted object alive until Ruby lets go of it.
pub struct EngineObject(Gd<Object>);

// SAFETY: a realm is entered by one thread at a time, so no two threads
// reach the object together, and the engine's objects are shared across
// threads under gdext's experimental-threads.
unsafe impl Send for EngineObject {}

static ENGINE_OBJECT: DataType<EngineObject> = DataType::new(c"Godot::Object");

// SAFETY: `define` marks Godot::Object, and every engine class under
// Godot descends from it.
unsafe impl TypedData for EngineObject {
    fn class(mrb: &Mrb) -> RClass {
        root(mrb).expect("the Godot gem defines Godot::Object")
    }

    fn data_type() -> &'static DataType<Self> {
        &ENGINE_OBJECT
    }
}

/// Defines Godot::Object, the class every engine class descends from.
pub fn define(mrb: &Mrb, godot: RModule) -> Result<(), Error> {
    let object = godot.define_class(mrb, c"Object", mrb.object_class())?;
    object.set_instance_data_tt(mrb)?;
    object.define_singleton_method(mrb, c"__make__", method!(make, 0))?;
    object.define_singleton_method(mrb, c"__make_node__", method!(make_node, 0))?;
    object.define_singleton_method(mrb, c"__allocate__", method!(allocate, 1))?;
    object.define_singleton_method(mrb, c"__singleton__", method!(singleton, 0))?;
    object.define_singleton_method(mrb, c"__static_method__", method!(static_method, 1))?;
    object.define_singleton_method(mrb, c"__call_static__", method!(call_static, 2))?;
    object.define_singleton_method(mrb, c"__engine_constant__", method!(engine_constant, 1))?;
    object.define_singleton_method(mrb, c"__declare_signal__", method!(declare_signal, 2))?;
    object.define_singleton_method(mrb, c"__declare_export__", method!(declare_export, 4))?;
    object.define_singleton_method(mrb, c"__declare_heading__", method!(declare_heading, 3))?;
    object.define_private_method(mrb, c"__resolve__", method!(resolve, 1))?;
    object.define_private_method(mrb, c"__call__", method!(call, 2))?;
    object.define_private_method(mrb, c"__instance_id__", method!(instance_id, 0))?;
    Ok(())
}

fn root(mrb: &Mrb) -> Result<RClass, Error> {
    mrb.module_get(c"Godot")?.class_get(mrb, c"Object")
}

// Godot::Object.__make__: a new engine object of the receiver's engine
// class.
fn make(mrb: &Mrb, class: RClass) -> Result<Value, Error> {
    let path = class.path(mrb).unwrap_or_default();
    let name = path.strip_prefix("Godot::").unwrap_or(&path);
    let class_db = ClassDb::singleton();
    if !class_db.can_instantiate(name) {
        let message = format!("the engine makes no objects of {path}");
        return Err(not_implemented(mrb, &message));
    }
    let object = class_db.instantiate(name).to::<Gd<Object>>();
    Ok(mrb.wrap_as(EngineObject(object), class).as_value())
}

// Godot::Object.__make_node__: a new node of the engine class the
// receiver's file extends, carrying that file's script, and the receiver's
// object for it, uninitialized and held under the node's key.
fn make_node(mrb: &Mrb, class: RClass) -> Result<Value, Error> {
    let path = class.path(mrb);
    let script = path
        .as_ref()
        .and_then(|path| {
            let names: Vec<String> = path.split("::").map(str::to_owned).collect();
            realm::file_defining(mrb, &names)
        })
        .and_then(|file| ResourceLoader::singleton().load(&file))
        .and_then(|resource| resource.try_cast::<Script>().ok())
        .filter(|script| !script.get_instance_base_type().is_empty());
    let class_name = || {
        path.clone()
            .unwrap_or_else(|| "an unnamed class".to_owned())
    };
    let Some(script) = script else {
        let message = format!(
            "no node script defines {}, so it makes no node",
            class_name()
        );
        return Err(not_implemented(mrb, &message));
    };
    let mut node = ClassDb::singleton()
        .instantiate(&script.get_instance_base_type())
        .to::<Gd<Object>>();
    node.set_script(&script);
    if node.get_script().is_none() {
        node.free();
        let message = format!("{}'s script refused its node here", class_name());
        return Err(not_implemented(mrb, &message));
    }
    let object = mrb.wrap_as(EngineObject(node.clone()), class).as_value();
    realm::hold(mrb, node_key(node.instance_id()), object)?;
    Ok(object)
}

// Godot::Object.__allocate__(owner): the receiver's object for the engine
// object `owner` stands for, uninitialized.
fn allocate(mrb: &Mrb, class: RClass, owner: &EngineObject) -> Value {
    mrb.wrap_as(EngineObject(owner.0.clone()), class).as_value()
}

/// The key a realm holds a node's Ruby object under.
pub fn node_key(node: InstanceId) -> Key {
    Key::from(node.to_i64())
}

/// A node's engine object as Ruby is given it, for its class to make the
/// node's Ruby object from.
pub struct Owner(pub InstanceId);

impl IntoValue for Owner {
    fn into_value(self, mrb: &Mrb) -> Value {
        match Gd::<Object>::try_from_instance_id(self.0) {
            Ok(owner) => mrb.wrap(EngineObject(owner)).as_value(),
            Err(_) => Value::nil(),
        }
    }
}

// Godot::Object#__resolve__(name): the engine method a call of `name`
// reaches, and whether the object's engine class declares it, so every
// object of the Ruby class answers it; nil when it reaches none, as for an
// object carrying no engine object. A name ending in `=` reaches the
// property's setter, and a name no method has reaches its getter.
fn resolve(mrb: &Mrb, receiver: Value, name: Symbol) -> Result<Value, Error> {
    let Ok(held) = <&EngineObject>::try_convert(receiver, mrb) else {
        return Ok(Value::nil());
    };
    let name = name.name(mrb).unwrap_or_default();
    let object = held.live(mrb, &name)?;
    let class = StringName::from(&object.get_class());
    let mut class_db = ClassDb::singleton();
    let (target, declared) = if let Some(property) = name.strip_suffix('=') {
        (
            class_db
                .class_get_property_setter(&class, property)
                .to_string(),
            true,
        )
    } else if object.has_method(name.as_str()) {
        let declared = class_db.class_has_method(&class, name.as_str());
        (name, declared)
    } else {
        (
            class_db
                .class_get_property_getter(&class, name.as_str())
                .to_string(),
            true,
        )
    };
    if target.is_empty() {
        return Ok(Value::nil());
    }
    let target = Symbol::from(mrb.intern(target.as_bytes())?).as_value();
    Ok(mrb
        .ary_new_from_values(&[target, declared.into_value(mrb)])
        .as_value())
}

// Godot::Object#__call__(name, args): calls the engine method `name` with
// `args` and answers what it returns.
fn call(mrb: &Mrb, held: &EngineObject, name: Symbol, args: Array) -> Result<Value, Error> {
    let name = name.name(mrb).unwrap_or_default();
    let mut object = held.live(mrb, &name)?;
    let args = variants(mrb, args)?;
    let answer = object
        .try_call(name.as_str(), &args)
        .map_err(|error| refused(mrb, &error, &object.get_class().to_string(), &name))?;
    answered(mrb, &answer)
}

// Godot::Object#__instance_id__: the engine object's instance id, which
// two Ruby objects for one engine object share.
fn instance_id(_mrb: &Mrb, held: &EngineObject) -> i64 {
    held.0.instance_id_unchecked().to_i64()
}

// Godot::Object.__singleton__: the engine's singleton of the receiver's
// engine class, or nil when the engine keeps none.
fn singleton(mrb: &Mrb, class: RClass) -> Value {
    let name = engine_name(mrb, class);
    Engine::singleton()
        .get_singleton(&name)
        .map(|object| mrb.wrap_as(EngineObject(object), class).as_value())
        .unwrap_or_else(Value::nil)
}

// Godot::Object.__static_method__(name): whether the receiver's engine class
// has a method of that name to call on the class.
fn static_method(mrb: &Mrb, class: RClass, name: Symbol) -> bool {
    let name = name.name(mrb).unwrap_or_default();
    ClassDb::singleton().class_has_method(&engine_name(mrb, class), name.as_str())
}

// Godot::Object.__call_static__(name, args): calls the static method `name`
// of the receiver's engine class with `args`.
fn call_static(mrb: &Mrb, class: RClass, name: Symbol, args: Array) -> Result<Value, Error> {
    let name = name.name(mrb).unwrap_or_default();
    let class = engine_name(mrb, class);
    let args = variants(mrb, args)?;
    let answer = ClassDb::singleton()
        .try_class_call_static(&class, name.as_str(), &args)
        .map_err(|error| refused(mrb, &error, &class, &name))?;
    answered(mrb, &answer)
}

// What the engine answered, as Ruby is given it, or the Godot::CallError an
// answer that cannot reach Ruby raises.
fn answered(mrb: &Mrb, answer: &Variant) -> Result<Value, Error> {
    ToRuby::checked(answer)
        .map(|answer| answer.into_value(mrb))
        .map_err(|reason| call_error(mrb, &reason))
}

/// The Ruby object for an engine object: the one the realm holds for the
/// node, or an object of its engine class under Godot.
pub fn ruby_object(mrb: &Mrb, object: Gd<Object>) -> Value {
    if let Some(held) = realm::held(mrb, node_key(object.instance_id())) {
        return held;
    }
    let class = mrb
        .module_get(c"Godot")
        .and_then(|godot| {
            let name = object.get_class().to_string();
            godot.as_value().const_get(mrb, name.as_str())
        })
        .ok()
        .and_then(RClass::from_value)
        .or_else(|| root(mrb).ok());
    match class {
        Some(class) => mrb.wrap_as(EngineObject(object), class).as_value(),
        None => Value::nil(),
    }
}

// Godot::Object.__declare_signal__(name, parameters): takes the signal the
// class declares as its body runs, for the realm to publish once the file
// has run.
fn declare_signal(
    mrb: &Mrb,
    class: RClass,
    name: String,
    parameters: Array,
) -> Result<Value, Error> {
    let parameters = parameters
        .entries(mrb)
        .filter_map(String::from_value)
        .collect();
    realm::declare_signal(mrb, class, Signal { name, parameters })?;
    Ok(Value::nil())
}

// Godot::Object.__declare_export__(name, default, hint, hint_string): takes
// the property the class exports as its body runs, its type read from the
// value it is declared with and its hint from the keyword naming it, for the
// realm to publish once the file has run. A declaration GDScript would
// refuse is refused in GDScript's own words, so the two languages read the
// same when the same mistake is made.
fn declare_export(
    mrb: &Mrb,
    class: RClass,
    name: String,
    default: Value,
    hint: String,
    hint_string: String,
) -> Result<Value, Error> {
    let default =
        value::to_engine(mrb, default, 1).map_err(|reason| argument_error(mrb, &reason))?;
    if let Some(engine_class) = engine_member(mrb, class, &name) {
        let message =
            format!("Member \"{name}\" redefined (original in native class '{engine_class}')");
        return Err(argument_error(mrb, &message));
    }
    let property = match hint.as_str() {
        "type" => typed(mrb, name, &default, &hint_string),
        _ => inferred(name, &default, &hint, hint_string),
    }
    .map_err(|reason| argument_error(mrb, &reason))?;
    realm::declare_export(mrb, class, property)?;
    Ok(Value::nil())
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
    Value::nil()
}

// The property an export naming its type declares: an object of the class
// it names, which Godot fills in from the scene or the project's files. The
// value it is declared with is the class's own to hold, so a value of
// another type is refused as GDScript refuses a mismatched one.
fn typed(mrb: &Mrb, name: String, default: &Variant, class: &str) -> Result<Property, String> {
    let (hint, class_name) = class_named(mrb, class)?;
    if let Some(given) = mismatched(mrb, default, class) {
        return Err(format!(
            "Cannot assign a value of type {given} to variable \"{name}\" with specified type {class_name}."
        ));
    }
    Ok(Property::new(name, default).of_class(hint, class_name))
}

// What the value an export was declared with is, unless the class it names
// takes it: nothing at all, which is what Godot fills in, or an object of
// that class, a class descending from it included.
fn mismatched(mrb: &Mrb, default: &Variant, class: &str) -> Option<String> {
    let kind = default.get_type();
    if kind == VariantType::NIL {
        return None;
    }
    let Ok(object) = default.try_to::<Gd<Object>>() else {
        return Some(type_name(kind));
    };
    (!is_of_class(mrb, &object, class)).then(|| class_of(&object))
}

// Whether `object` is of the class `class` names: the engine's class
// database answers for an engine class, and a node is of a Ruby class when
// its script is that class's file or inherits from it.
fn is_of_class(mrb: &Mrb, object: &Gd<Object>, class: &str) -> bool {
    if let Some(engine_class) = class.strip_prefix("Godot::") {
        let object_class = StringName::from(&object.get_class());
        return ClassDb::singleton().is_parent_class(&object_class, engine_class);
    }
    let names: Vec<String> = class.split("::").map(str::to_owned).collect();
    let Some(file) = realm::file_defining(mrb, &names) else {
        return false;
    };
    written_in(object).any(|path| path == file)
}

// The name `object`'s own class is known by: the one its node script is
// announced under, or the engine class it is of.
fn class_of(object: &Gd<Object>) -> String {
    written_in(object)
        .next()
        .and_then(|path| editor_name_at(&path))
        .unwrap_or_else(|| object.get_class().to_string())
}

// The files `object`'s script is written in, nearest first: its own and the
// ones it inherits from, as Godot answers for them.
fn written_in(object: &Gd<Object>) -> impl Iterator<Item = String> {
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
fn inferred(
    name: String,
    default: &Variant,
    hint: &str,
    hint_string: String,
) -> Result<Property, String> {
    if default.get_type() == VariantType::NIL {
        return Err(
            "Cannot use \"export\" because the type of the initialized value can't be inferred."
                .to_owned(),
        );
    }
    let hint = hint_named(hint, default.get_type())?;
    Ok(Property::new(name, default).hinted(hint, hint_string))
}

// The hint an exported object takes from the class it names, and the name
// the editor reads that class by: a resource is chosen among the project's
// files and a node among the scene's, while a class of neither kind is no
// type an export can take. A Ruby class is named by the announcement the
// editor lists it under, so one no announcement names is out of reach.
fn class_named(mrb: &Mrb, class: &str) -> Result<(PropertyHint, String), String> {
    if let Some(engine_class) = class.strip_prefix("Godot::") {
        let class_db = ClassDb::singleton();
        if !class_db.class_exists(engine_class) {
            return Err(unnamed(class));
        }
        if class_db.is_parent_class(engine_class, "Resource") {
            return Ok((PropertyHint::RESOURCE_TYPE, engine_class.to_owned()));
        }
        if class_db.is_parent_class(engine_class, "Node") {
            return Ok((PropertyHint::NODE_TYPE, engine_class.to_owned()));
        }
        return Err("Export type can only be built-in, a resource, a node, or an enum.".to_owned());
    }
    editor_name(mrb, class)
        .map(|name| (PropertyHint::NODE_TYPE, name))
        .ok_or_else(|| unnamed(class))
}

// The name the editor lists the Ruby class `class` under, if a node script
// of the project defines it and nothing else is announced by that name.
fn editor_name(mrb: &Mrb, class: &str) -> Option<String> {
    let names: Vec<String> = class.split("::").map(str::to_owned).collect();
    editor_name_at(&realm::file_defining(mrb, &names)?)
}

// The name the editor lists the file at `path` under, if it is announced.
fn editor_name_at(path: &str) -> Option<String> {
    let test_directories = settings::test_directories();
    let project = Project::new(&FilesOnDisk, &test_directories, &super::is_node_class);
    project
        .announce(path)
        .ok()
        .map(|announcement| announcement.name)
}

// GDScript's words for a type nothing in the project is known by.
fn unnamed(class: &str) -> String {
    format!("The class \"{class}\" was not found in the global scope.")
}

// The hint the keyword `name` spells, unless the exported type cannot be
// read with it. Each one takes the types the `@export_*` annotation it
// answers to takes, and a type none of them takes is refused in GDScript's
// words. Ruby names the hint, so a name none of them spells is no hint.
fn hint_named(name: &str, kind: VariantType) -> Result<PropertyHint, String> {
    let (hint, takes): (PropertyHint, &[VariantType]) = match name {
        "range" => (PropertyHint::RANGE, &[VariantType::INT, VariantType::FLOAT]),
        "enum" => (
            PropertyHint::ENUM,
            &[
                VariantType::INT,
                VariantType::STRING,
                VariantType::STRING_NAME,
            ],
        ),
        "flags" => (PropertyHint::FLAGS, &[VariantType::INT]),
        "file" => (PropertyHint::FILE, &[VariantType::STRING]),
        "dir" => (PropertyHint::DIR, &[VariantType::STRING]),
        "multiline" => (PropertyHint::MULTILINE_TEXT, &[VariantType::STRING]),
        "placeholder" => (PropertyHint::PLACEHOLDER_TEXT, &[VariantType::STRING]),
        _ => return Ok(PropertyHint::NONE),
    };
    if takes.contains(&kind) {
        return Ok(hint);
    }
    Err(format!(
        "\"{name}:\" requires a variable of type {}, but type \"{}\" was given instead.",
        kind_list(takes),
        type_name(kind)
    ))
}

// The types a hint takes, as GDScript lists them in the same refusal: the
// last is reached through "or", and three or more are separated by commas.
fn kind_list(kinds: &[VariantType]) -> String {
    let names: Vec<String> = kinds
        .iter()
        .map(|kind| format!("\"{}\"", type_name(*kind)))
        .collect();
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, [first])) => format!("{first} or {last}"),
        Some((last, rest)) => format!("{}, or {last}", rest.join(", ")),
    }
}

// The name GDScript gives a variant type.
fn type_name(kind: VariantType) -> String {
    type_string(i64::from(kind.ord())).to_string()
}

// The engine class `class` extends that has a member of that name, if one
// has: a signal, a property or an integer constant, which is what GDScript
// refuses a member for redefining.
fn engine_member(mrb: &Mrb, class: RClass, name: &str) -> Option<String> {
    let engine_class = engine_ancestor(mrb, class)?;
    let mut class_db = ClassDb::singleton();
    let has = class_db.class_has_signal(engine_class.as_str(), name)
        || class_db.class_has_integer_constant(engine_class.as_str(), name)
        || !class_db
            .class_get_property_setter(engine_class.as_str(), name)
            .is_empty()
        || !class_db
            .class_get_property_getter(engine_class.as_str(), name)
            .is_empty();
    has.then_some(engine_class)
}

// The engine class `class` extends, itself or through its superclasses, as
// the engine names it.
fn engine_ancestor(mrb: &Mrb, class: RClass) -> Option<String> {
    let mut current = class.as_value();
    loop {
        let path = RClass::from_value(current)?.path(mrb)?;
        if let Some(name) = path.strip_prefix("Godot::") {
            return Some(name.to_owned());
        }
        current = current.funcall(mrb, c"superclass", &[]).ok()?;
    }
}

// Godot::Object.__engine_constant__(name): the integer constant or enum
// value of that name the receiver's engine class has, or nil.
fn engine_constant(mrb: &Mrb, class: RClass, name: Symbol) -> Value {
    let name = name.name(mrb).unwrap_or_default();
    let class = engine_name(mrb, class);
    let class_db = ClassDb::singleton();
    if !class_db.class_has_integer_constant(&class, name.as_str()) {
        return Value::nil();
    }
    class_db
        .class_get_integer_constant(&class, name.as_str())
        .into_value(mrb)
}

// The engine class an engine class under Godot stands for.
fn engine_name(mrb: &Mrb, class: RClass) -> String {
    let path = class.path(mrb).unwrap_or_default();
    path.strip_prefix("Godot::").unwrap_or(&path).to_owned()
}

// `args` as the engine takes them, or the Godot::CallError one that cannot
// reach it raises, before the engine is given any.
fn variants(mrb: &Mrb, args: Array) -> Result<Vec<Variant>, Error> {
    args.entries(mrb)
        .map(|arg| value::to_engine(mrb, arg, 1).map_err(|reason| call_error(mrb, &reason)))
        .collect()
}

impl EngineObject {
    /// The engine object as the engine takes it, unless it was freed.
    pub fn variant(&self) -> Result<Variant, String> {
        if self.0.is_instance_valid() {
            Ok(self.0.to_variant())
        } else {
            Err("a freed engine object cannot reach the engine".to_owned())
        }
    }

    // The engine object, or the Godot::CallError calling `method` on a freed
    // one raises, worded as GDScript words it.
    fn live(&self, mrb: &Mrb, method: &str) -> Result<Gd<Object>, Error> {
        if self.0.is_instance_valid() {
            Ok(self.0.clone())
        } else {
            let message = format!(
                "Attempt to call function '{method}' in base 'previously freed' on a null instance."
            );
            Err(call_error(mrb, &message))
        }
    }
}

// The Godot::CallError a call the engine refused raises, worded as GDScript
// words the same failure of an untyped call. gdext hands the engine's
// reason only as text, so the argument and types are read back from it.
fn refused(mrb: &Mrb, error: &CallError, base: &str, method: &str) -> Error {
    let reason = error.message(false);
    let reason = reason.rsplit("Reason: ").next().unwrap_or_default();
    let message = if let Some(expected) = expected_count(reason) {
        format!(
            "Invalid call to function '{method}' in base '{base}'. Expected {expected} argument(s)."
        )
    } else if let Some((argument, from, to)) = conversion(reason) {
        format!(
            "Invalid type in function '{method}' in base '{base}'. \
             Cannot convert argument {argument} from {from} to {to}."
        )
    } else {
        format!("Invalid call to function '{method}' in base '{base}': {reason}")
    };
    call_error(mrb, &message)
}

/// GDScript's words for a call given the wrong number of arguments.
pub(super) fn count_message(method: &str, base: &str, expected: &str) -> String {
    format!(
        "Invalid call to function '{method}' in base '{base}'. Expected {expected} argument(s)."
    )
}

/// GDScript's words for a call given an argument of a type it cannot take.
pub(super) fn type_message(
    method: &str,
    base: &str,
    argument: &str,
    from: &str,
    to: &str,
) -> String {
    format!(
        "Invalid type in function '{method}' in base '{base}'. \
         Cannot convert argument {argument} from {from} to {to}."
    )
}

// "function has N parameters, but received M arguments": N.
fn expected_count(reason: &str) -> Option<&str> {
    reason
        .strip_prefix("function has ")?
        .split_once(" parameter")
        .map(|(count, _)| count)
}

// "parameter #I -- cannot convert from FROM to TO": I and the two types as
// GDScript names them.
fn conversion(reason: &str) -> Option<(&str, String, String)> {
    let (argument, types) = reason
        .strip_prefix("parameter #")?
        .split_once(" -- cannot convert from ")?;
    let (from, to) = types.split_once(" to ")?;
    Some((argument, type_named(from)?, type_named(to)?))
}

// The name GDScript gives the variant type gdext debug-prints as `debug`.
fn type_named(debug: &str) -> Option<String> {
    (0..VariantType::MAX.ord)
        .map(<VariantType as EngineEnum>::from_ord)
        .find(|kind| format!("{kind:?}") == debug)
        .map(|kind| type_string(i64::from(kind.ord)).to_string())
}

fn argument_error(mrb: &Mrb, message: &str) -> Error {
    match mrb.exc_get(c"ArgumentError") {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}

fn not_implemented(mrb: &Mrb, message: &str) -> Error {
    match mrb.exc_get(c"NotImplementedError") {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}

pub(super) fn call_error(mrb: &Mrb, message: &str) -> Error {
    let class = mrb
        .module_get(c"Godot")
        .and_then(|godot| godot.class_get(mrb, c"CallError"));
    match class.map(|class| ExceptionClass::from_value(class.as_value())) {
        Ok(Some(class)) => Error::new(mrb, class, message),
        Ok(None) => unreachable!("Godot::CallError is an exception class"),
        Err(error) => error,
    }
}
