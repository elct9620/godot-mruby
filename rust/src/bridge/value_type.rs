//! The engine's value types in Ruby: `Vector2`, `Color`, `NodePath` and the
//! rest are classes under `Godot` descending from `Godot::Value`, whose
//! values are built, read and computed with by the engine itself, and never
//! change. gdext answers only a statically typed side of these types, so the
//! engine's own variant calls do the work, by name, for every type alike.

use std::ptr;

use beni::{
    Array, DataType, Error, IntoValue, Module, Mrb, Object as _, RClass, RModule, ReprValue,
    Symbol, TypedData, Value, method,
};
use godot::builtin::{GString, StringName, Variant, VariantOperator, VariantType};
use godot::global::type_string;
use godot::obj::EngineEnum;
use godot::sys;

use super::value::{self, ToRuby};

/// A value of one of the engine's value types, as Ruby holds it.
pub struct EngineValue(Variant);

// SAFETY: a value type holds no object and no reference into the engine
// that another thread could change, and the realm holding it is entered by
// one thread at a time.
unsafe impl Send for EngineValue {}

static ENGINE_VALUE: DataType<EngineValue> = DataType::new(c"Godot::Value");

// SAFETY: `define` marks Godot::Value, and every value type's class under
// Godot descends from it.
unsafe impl TypedData for EngineValue {
    fn class(mrb: &Mrb) -> RClass {
        mrb.module_get(c"Godot")
            .and_then(|godot| godot.class_get(mrb, c"Value"))
            .expect("the Godot gem defines Godot::Value")
    }

    fn data_type() -> &'static DataType<Self> {
        &ENGINE_VALUE
    }
}

/// The engine's value types Ruby holds as `Godot::Value`s: every type that
/// is neither Ruby's own (nil, booleans, numbers, strings, names,
/// containers) nor an object, a callable or a signal.
fn value_types() -> impl Iterator<Item = VariantType> {
    (VariantType::VECTOR2.ord..=VariantType::COLOR.ord)
        .chain([VariantType::NODE_PATH.ord, VariantType::RID.ord])
        .map(<VariantType as EngineEnum>::from_ord)
}

/// Whether the engine's values of `kind` reach Ruby as `Godot::Value`s.
pub fn is_value_type(kind: VariantType) -> bool {
    value_types().any(|value_type| value_type == kind)
}

fn type_name(kind: VariantType) -> String {
    type_string(i64::from(kind.ord)).to_string()
}

/// Defines Godot::Value, the class every value type's class descends from.
pub fn define(mrb: &Mrb, godot: RModule) -> Result<(), Error> {
    let class = godot.define_class(mrb, c"Value", mrb.object_class())?;
    class.set_instance_data_tt(mrb)?;
    class.define_singleton_method(mrb, c"__value_type__", method!(value_type_named, 1))?;
    class.define_singleton_method(mrb, c"__construct__", method!(construct, 1))?;
    class.define_singleton_method(mrb, c"__call_static__", method!(call_static, 2))?;
    class.define_singleton_method(mrb, c"__constant__", method!(constant, 1))?;
    class.define_private_method(mrb, c"__member__", method!(member, 1))?;
    class.define_private_method(mrb, c"__call__", method!(call, 2))?;
    class.define_private_method(mrb, c"__operate__", method!(operate, 2))?;
    class.define_private_method(mrb, c"__hash__", method!(hash, 0))?;
    class.define_method(mrb, c"to_s", method!(to_s, 0))?;
    Ok(())
}

/// The Ruby value for a value of a value type: a value of its class under
/// Godot.
pub fn ruby_value(mrb: &Mrb, variant: &Variant) -> Value {
    let class = mrb
        .module_get(c"Godot")
        .and_then(|godot| godot.class_get(mrb, type_name(variant.get_type()).as_str()));
    match class {
        Ok(class) => mrb.wrap_as(EngineValue(variant.clone()), class).as_value(),
        Err(_) => Value::nil(),
    }
}

impl EngineValue {
    /// The value as the engine takes it.
    pub fn variant(&self) -> Variant {
        self.0.clone()
    }
}

// Godot::Value.__value_type__(name): whether the engine has a value type of
// that name for Ruby to hold.
fn value_type_named(mrb: &Mrb, _class: Value, name: Symbol) -> bool {
    let name = name.name(mrb).unwrap_or_default();
    kind_named(&name).is_some()
}

fn kind_named(name: &str) -> Option<VariantType> {
    value_types().find(|kind| type_name(*kind) == name)
}

// The value type a class under Godot stands for.
fn kind_of(mrb: &Mrb, class: RClass) -> Result<VariantType, Error> {
    let path = class.path(mrb).unwrap_or_default();
    let name = path.strip_prefix("Godot::").unwrap_or(&path);
    kind_named(name).ok_or_else(|| {
        let message = format!("{path} is no value type of the engine");
        match mrb.exc_get(c"TypeError") {
            Ok(type_error) => Error::new(mrb, type_error, &message),
            Err(error) => error,
        }
    })
}

// Godot::Value.__construct__(args): the value the engine's constructor of
// the receiver's type that takes `args` builds.
fn construct(mrb: &Mrb, class: RClass, args: Array) -> Result<Value, Error> {
    let kind = kind_of(mrb, class)?;
    let args = variants(mrb, args)?;
    let pointers: Vec<_> = args.iter().map(Variant::var_sys).collect();
    let kind_sys = kind.ord as sys::GDExtensionVariantType;
    // SAFETY: the argument pointers live as long as `args`.
    let built = engine_call(|built, error| unsafe {
        sys::interface_fn!(variant_construct)(
            kind_sys,
            built,
            pointers.as_ptr(),
            pointers.len() as i32,
            error,
        )
    });
    match built {
        Ok(built) => Ok(ruby_value(mrb, &built)),
        Err(_) => {
            let message = format!(
                "Invalid call. Nonexistent '{}' constructor.",
                type_name(kind)
            );
            Err(super::object::call_error(mrb, &message))
        }
    }
}

// Godot::Value#__member__(name): the member of that name in a one-element
// array, or nil when the value has no such member.
fn member(mrb: &Mrb, held: &EngineValue, name: Symbol) -> Value {
    let name = StringName::from(name.name(mrb).unwrap_or_default().as_str());
    let kind = held.0.get_type().ord as sys::GDExtensionVariantType;
    // SAFETY: the interface is initialized while the extension runs, and the
    // name lives for the call.
    if unsafe { sys::interface_fn!(variant_has_member)(kind, name.string_sys()) } == 0 {
        return Value::nil();
    }
    // SAFETY: the interface is initialized while the extension runs, and the
    // type has the member, which `variant_get_named` writes.
    let found = unsafe {
        Variant::new_with_var_uninit_result(|found| {
            let mut valid = false as sys::GDExtensionBool;
            sys::interface_fn!(variant_get_named)(
                held.0.var_sys(),
                name.string_sys(),
                found,
                ptr::addr_of_mut!(valid),
            );
            if valid == 0 { Err(()) } else { Ok(()) }
        })
    };
    match found {
        Ok(found) => mrb.ary_new_from_values(&[to_ruby(mrb, &found)]).as_value(),
        Err(()) => Value::nil(),
    }
}

// Godot::Value#__call__(name, args): the engine method `name` of the value
// called with `args`, in a one-element array, or nil when the type has no
// such method.
fn call(mrb: &Mrb, held: &EngineValue, name: Symbol, args: Array) -> Result<Value, Error> {
    let method = name.name(mrb).unwrap_or_default();
    let args = variants(mrb, args)?;
    let pointers: Vec<_> = args.iter().map(Variant::var_sys).collect();
    let name = StringName::from(method.as_str());
    let mut receiver = held.0.clone();
    // SAFETY: the name and argument pointers live for the call, which runs on
    // a copy of the value, so the Ruby value never changes.
    let answered = engine_call(|answer, error| unsafe {
        sys::interface_fn!(variant_call)(
            receiver.var_sys_mut(),
            name.string_sys(),
            pointers.as_ptr(),
            pointers.len() as i64,
            answer,
            error,
        )
    });
    wrap_answer(mrb, answered, &type_name(held.0.get_type()), &method, &args)
}

// Godot::Value.__call_static__(name, args): the static method `name` of the
// receiver's type called with `args`, in a one-element array, or nil when
// the type has no such method.
fn call_static(mrb: &Mrb, class: RClass, name: Symbol, args: Array) -> Result<Value, Error> {
    let kind = kind_of(mrb, class)?;
    let method = name.name(mrb).unwrap_or_default();
    let args = variants(mrb, args)?;
    let pointers: Vec<_> = args.iter().map(Variant::var_sys).collect();
    let name = StringName::from(method.as_str());
    let kind_sys = kind.ord as sys::GDExtensionVariantType;
    // SAFETY: the name and argument pointers live for the call.
    let answered = engine_call(|answer, error| unsafe {
        sys::interface_fn!(variant_call_static)(
            kind_sys,
            name.string_sys(),
            pointers.as_ptr(),
            pointers.len() as i64,
            answer,
            error,
        )
    });
    wrap_answer(mrb, answered, &type_name(kind), &method, &args)
}

// Runs an engine call that writes its answer into an uninitialized variant
// and reports failure through a call error, and answers either.
fn engine_call(
    call: impl FnOnce(sys::GDExtensionUninitializedVariantPtr, *mut sys::GDExtensionCallError),
) -> Result<Variant, sys::GDExtensionCallError> {
    // SAFETY: the interface is initialized while the extension runs, and an
    // engine call writes the answer whenever it leaves the error at CALL_OK.
    unsafe {
        Variant::new_with_var_uninit_result(|answer| {
            let mut error = sys::default_call_error();
            call(answer, ptr::addr_of_mut!(error));
            if error.error == sys::GDEXTENSION_CALL_OK {
                Ok(())
            } else {
                Err(error)
            }
        })
    }
}

// What a call answered, in a one-element array; nil for a method the type
// lacks, and the Godot::CallError GDScript's wording gives any other
// failure.
fn wrap_answer(
    mrb: &Mrb,
    answered: Result<Variant, sys::GDExtensionCallError>,
    base: &str,
    method: &str,
    args: &[Variant],
) -> Result<Value, Error> {
    match answered {
        Ok(answer) => Ok(mrb.ary_new_from_values(&[to_ruby(mrb, &answer)]).as_value()),
        Err(error) if error.error == sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD => Ok(Value::nil()),
        Err(error) => {
            let message = match error.error {
                sys::GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT => {
                    let index = error.argument as usize;
                    let from = args
                        .get(index)
                        .map_or_else(String::new, |arg| type_name(arg.get_type()));
                    let to = type_name(<VariantType as EngineEnum>::from_ord(error.expected));
                    super::object::type_message(method, base, &(index + 1).to_string(), &from, &to)
                }
                _ => super::object::count_message(method, base, &error.expected.to_string()),
            };
            Err(super::object::call_error(mrb, &message))
        }
    }
}

// Godot::Value.__constant__(name): the receiver's type's constant of that
// name, or nil when it has none.
fn constant(mrb: &Mrb, class: RClass, name: Symbol) -> Result<Value, Error> {
    let kind = kind_of(mrb, class)?;
    let name = StringName::from(name.name(mrb).unwrap_or_default().as_str());
    // SAFETY: the interface is initialized while the extension runs, and the
    // engine writes Nil for a name the type has no constant of.
    let found = unsafe {
        Variant::new_with_var_uninit_result(|found| {
            sys::interface_fn!(variant_get_constant_value)(
                kind.ord as sys::GDExtensionVariantType,
                name.string_sys(),
                found,
            );
            Ok::<(), ()>(())
        })
    };
    Ok(match found {
        Ok(found) if !found.is_nil() => to_ruby(mrb, &found),
        _ => Value::nil(),
    })
}

// Godot::Value#__operate__(operator, other): what the engine's operator
// answers for the value and `other`, or the TypeError GDScript's wording
// gives an operator the engine lacks for them.
fn operate(mrb: &Mrb, held: &EngineValue, operator: Symbol, other: Value) -> Result<Value, Error> {
    let symbol = operator.name(mrb).unwrap_or_default();
    let (op, shown) = match symbol.as_str() {
        "+" => (VariantOperator::ADD, "+"),
        "-" => (VariantOperator::SUBTRACT, "-"),
        "*" => (VariantOperator::MULTIPLY, "*"),
        "/" => (VariantOperator::DIVIDE, "/"),
        "%" => (VariantOperator::MODULO, "%"),
        "**" => (VariantOperator::POWER, "**"),
        "==" => (VariantOperator::EQUAL, "=="),
        "<" => (VariantOperator::LESS, "<"),
        "<=" => (VariantOperator::LESS_EQUAL, "<="),
        ">" => (VariantOperator::GREATER, ">"),
        ">=" => (VariantOperator::GREATER_EQUAL, ">="),
        "-@" => (VariantOperator::NEGATE, "unary-"),
        "+@" => (VariantOperator::POSITIVE, "unary+"),
        _ => (VariantOperator::MAX, symbol.as_str()),
    };
    let other = value::to_engine(mrb, other, 1).map_err(|reason| type_error(mrb, &reason))?;
    match Variant::evaluate(&held.0, &other, op) {
        Some(answer) if op != VariantOperator::MAX => Ok(to_ruby(mrb, &answer)),
        _ => {
            let message = format!(
                "Invalid operands '{}' and '{}' in operator '{shown}'.",
                type_name(held.0.get_type()),
                type_name(other.get_type())
            );
            Err(type_error(mrb, &message))
        }
    }
}

// Godot::Value#__hash__: the engine's hash of the value, which equal values
// share.
fn hash(_mrb: &Mrb, held: &EngineValue) -> i64 {
    // SAFETY: the interface is initialized while the extension runs, and
    // the value lives for the call.
    unsafe { sys::interface_fn!(variant_hash)(held.0.var_sys()) }
}

// Godot::Value#to_s: the value as the engine prints it.
fn to_s(mrb: &Mrb, held: &EngineValue) -> Value {
    let printed = GString::from(&held.0.to_string()).to_string();
    mrb.str_new(printed.as_bytes()).as_value()
}

// What the engine answered for a value, as Ruby is given it; a value type's
// answers hold no container Ruby could not take.
fn to_ruby(mrb: &Mrb, answer: &Variant) -> Value {
    ToRuby::checked(answer).map_or_else(|_| Value::nil(), |answer| answer.into_value(mrb))
}

fn variants(mrb: &Mrb, args: Array) -> Result<Vec<Variant>, Error> {
    args.entries(mrb)
        .map(|arg| {
            value::to_engine(mrb, arg, 1).map_err(|reason| super::object::call_error(mrb, &reason))
        })
        .collect()
}

fn type_error(mrb: &Mrb, message: &str) -> Error {
    match mrb.exc_get(c"TypeError") {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}
