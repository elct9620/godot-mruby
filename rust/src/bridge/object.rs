//! The engine's objects as Ruby holds them: an object of a class under
//! `Godot` carries the engine object it stands for, and Ruby reaches its
//! methods by their names.

use beni::{
    Array, DataType, Error, ExceptionClass, FromValue, IntoValue, Module, Mrb, Object as _, RClass,
    RModule, ReprValue, Symbol, TryConvert, TypedData, Value, method,
};
use godot::classes::{ClassDb, Object};
use godot::obj::{Gd, Singleton};

use super::{Answer, Argument};

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
        let not_implemented = mrb.exc_get(c"NotImplementedError")?;
        let message = format!("the engine makes no objects of {path}");
        return Err(Error::new(mrb, not_implemented, &message));
    }
    let object = class_db.instantiate(name).to::<Gd<Object>>();
    Ok(mrb.wrap_as(EngineObject(object), class).as_value())
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
