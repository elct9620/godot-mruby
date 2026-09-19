//! The engine's objects as Ruby holds them: an object of a class under
//! `Godot` carries the engine object it stands for, and Ruby reaches its
//! methods by their names.

use beni::{
    Array, DataType, Error, ExceptionClass, FromValue, IntoValue, Module, Mrb, Object as _, RClass,
    RModule, ReprValue, Symbol, TryConvert, TypedData, Value, method,
};
use godot::classes::{ClassDb, Object, ResourceLoader, Script};
use godot::obj::{Gd, InstanceId, Singleton};

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
    object.define_private_method(mrb, c"__engine_method__", method!(engine_method, 1))?;
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

// Godot::Object#__engine_method__(name): whether the engine object has a
// method of that name; false for an object carrying none.
fn engine_method(mrb: &Mrb, receiver: Value, name: Symbol) -> Result<bool, Error> {
    let Ok(held) = <&EngineObject>::try_convert(receiver, mrb) else {
        return Ok(false);
    };
    let object = held.live(mrb)?;
    Ok(name
        .name(mrb)
        .is_some_and(|name| object.has_method(name.as_str())))
}

// Godot::Object#__call__(name, args): calls the engine method `name` with
// `args` and answers what it returns.
fn call(mrb: &Mrb, held: &EngineObject, name: Symbol, args: Array) -> Result<Value, Error> {
    let mut object = held.live(mrb)?;
    let name = name.name(mrb).unwrap_or_default();
    let args = (0..args.len())
        .map(|index| Answer::try_convert(args.entry(mrb, index as isize), mrb).map(|Answer(v)| v))
        .collect::<Result<Vec<_>, _>>()?;
    let answer = object
        .try_call(name.as_str(), &args)
        .map_err(|error| call_error(mrb, &error.to_string()))?;
    Ok(Argument(&answer).into_value(mrb))
}

impl EngineObject {
    // The engine object, or the Godot::CallError a freed one raises.
    fn live(&self, mrb: &Mrb) -> Result<Gd<Object>, Error> {
        if self.0.is_instance_valid() {
            Ok(self.0.clone())
        } else {
            Err(call_error(mrb, "the engine object was freed"))
        }
    }
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
