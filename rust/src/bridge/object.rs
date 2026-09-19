//! The engine's objects as Ruby holds them: an object of a class under
//! `Godot` carries the engine object it stands for, and Ruby reaches its
//! methods by their names.

use beni::{
    Array, DataType, Error, ExceptionClass, FromValue, IntoValue, Module, Mrb, Object as _, RClass,
    RModule, ReprValue, Symbol, TryConvert, TypedData, Value, method,
};
use godot::builtin::StringName;
use godot::builtin::VariantType;
use godot::classes::{ClassDb, Engine, Object, ResourceLoader, Script};
use godot::global::type_string;
use godot::meta::error::CallError;
use godot::obj::{EngineEnum, Gd, InstanceId, Singleton};

use super::{Answer, Argument};
use crate::realm::{self, Key};

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
    object.define_private_method(mrb, c"__resolve__", method!(resolve, 1))?;
    object.define_private_method(mrb, c"__call__", method!(call, 2))?;
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
    Ok(Argument(&answer).into_value(mrb))
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
    Ok(Argument(&answer).into_value(mrb))
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

fn variants(mrb: &Mrb, args: Array) -> Result<Vec<godot::builtin::Variant>, Error> {
    (0..args.len())
        .map(|index| Answer::try_convert(args.entry(mrb, index as isize), mrb).map(|Answer(v)| v))
        .collect()
}

impl EngineObject {
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

fn not_implemented(mrb: &Mrb, message: &str) -> Error {
    match mrb.exc_get(c"NotImplementedError") {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}

fn call_error(mrb: &Mrb, message: &str) -> Error {
    let class = mrb
        .module_get(c"Godot")
        .and_then(|godot| godot.class_get(mrb, c"CallError"));
    match class.map(|class| ExceptionClass::from_value(class.as_value())) {
        Ok(Some(class)) => Error::new(mrb, class, message),
        Ok(None) => unreachable!("Godot::CallError is an exception class"),
        Err(error) => error,
    }
}
