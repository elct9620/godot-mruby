//! The engine's value types in Ruby: `Vector2`, `Color`, `NodePath` and the
//! rest are classes under `Godot` descending from `Godot::Value`, whose
//! values are built, read and computed with by the engine itself, and never
//! change. gdext answers only a statically typed side of these types, so the
//! engine's own variant calls do the work, by name, for every type alike.

use std::cell::{Cell, RefCell};
use std::ptr;

use beni::{
    DataType, Error, IntoValue, Module, Mrb, Object as _, RArray, RClass, RModule, ReprValue,
    Symbol, TypedData, Value, method, value::qnil,
};
use godot::builtin::{GString, StringName, Variant, VariantOperator, VariantType};
use godot::obj::EngineEnum;
use godot::sys;
use smallvec::SmallVec;

use super::object::{to_arguments, type_error};
use super::value::{self, ToRuby};
use crate::hint::type_name;
use crate::realm;

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
        let kept = &realm::extension_data::<ValueClass>(mrb).0;
        super::find_class_once(mrb, kept, || {
            mrb.module_get(c"Godot")
                .and_then(|godot| godot.class_get(mrb, c"Value"))
                .expect("the Godot gem defines Godot::Value")
        })
    }

    fn data_type() -> &'static DataType<Self> {
        &ENGINE_VALUE
    }
}

// Godot::Value, found once for a realm and kept for it.
#[derive(Default)]
struct ValueClass(Cell<Option<RClass>>);

/// The engine's value types Ruby holds as `Godot::Value`s: every type that
/// is neither Ruby's own (nil, booleans, numbers, strings, names,
/// containers) nor an object.
fn value_types() -> impl Iterator<Item = VariantType> {
    (VariantType::VECTOR2.ord..=VariantType::COLOR.ord)
        .chain([
            VariantType::NODE_PATH.ord,
            VariantType::RID.ord,
            VariantType::CALLABLE.ord,
            VariantType::SIGNAL.ord,
        ])
        .map(<VariantType as EngineEnum>::from_ord)
}

/// Whether the engine's values of `kind` reach Ruby as `Godot::Value`s.
pub fn is_value_type(kind: VariantType) -> bool {
    value_types().any(|value_type| value_type == kind)
}

/// Defines Godot::Value, the class every value type's class descends from.
pub fn define(mrb: &Mrb, godot: RModule) -> Result<(), Error> {
    let class = godot.define_class(mrb, c"Value", mrb.object_class())?;
    class.set_instance_data_tt(mrb)?;
    class.define_singleton_method(mrb, c"__has_value_type__", method!(has_value_type, 1))?;
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
    match class_by_kind(mrb, variant.get_type()) {
        Some(class) => mrb.wrap_as(EngineValue(variant.clone()), class).as_value(),
        None => qnil().as_value(),
    }
}

// The class under Godot of each value type a realm has handed Ruby a value
// of, by the type's ordinal, so a value finds its class without spelling the
// type's name.
#[derive(Default)]
struct ValueClasses(RefCell<Vec<Option<RClass>>>);

// The class under Godot of the value type `kind`, found once for a realm and
// kept for it, rooted for the collector so the class lives while it is kept.
fn class_by_kind(mrb: &Mrb, kind: VariantType) -> Option<RClass> {
    let classes = &realm::extension_data::<ValueClasses>(mrb).0;
    let index = kind.ord as usize;
    if let Some(class) = classes.borrow().get(index).copied().flatten() {
        return Some(class);
    }
    let class = mrb
        .module_get(c"Godot")
        .and_then(|godot| godot.class_get(mrb, type_name(kind).as_str()))
        .ok()?;
    mrb.gc_register_forever(class.as_value());
    let mut classes = classes.borrow_mut();
    if classes.len() <= index {
        classes.resize(index + 1, None);
    }
    classes[index] = Some(class);
    Some(class)
}

impl EngineValue {
    /// The value as the engine takes it.
    pub fn variant(&self) -> Variant {
        self.0.clone()
    }
}

// Godot::Value.__has_value_type__(name): whether the engine has a value type of
// that name for Ruby to hold.
fn has_value_type(mrb: &Mrb, _class: Value, name: Symbol) -> bool {
    let name = name.name(mrb).unwrap_or_default();
    kind_by_name(&name).is_some()
}

fn kind_by_name(name: &str) -> Option<VariantType> {
    value_types().find(|kind| type_name(*kind) == name)
}

// The value type a class under Godot stands for.
fn kind_by_class(mrb: &Mrb, class: RClass) -> Result<VariantType, Error> {
    let path = class.path(mrb).unwrap_or_default();
    let name = path.strip_prefix("Godot::").unwrap_or(&path);
    kind_by_name(name)
        .ok_or_else(|| type_error(mrb, &format!("{path} is no value type of the engine")))
}

// Godot::Value.__construct__(args): the value the engine's constructor of
// the receiver's type that takes `args` builds.
fn construct(mrb: &Mrb, class: RClass, args: RArray) -> Result<Value, Error> {
    let kind = kind_by_class(mrb, class)?;
    let args = to_arguments(mrb, args)?;
    let pointers: SmallVec<[_; 4]> = args.iter().map(Variant::var_sys).collect();
    let kind_sys = kind.ord as sys::GDExtensionVariantType;
    // SAFETY: the argument pointers live as long as `args`.
    let built = super::run_engine_call(|built, error| unsafe {
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
        return qnil().as_value();
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
        Err(()) => qnil().as_value(),
    }
}

// Godot::Value#__call__(name, args): the engine method `name` of the value
// called with `args`, in a one-element array, or nil when the type has no
// such method.
fn call(mrb: &Mrb, held: &EngineValue, name: Symbol, args: RArray) -> Result<Value, Error> {
    let method = name.name(mrb).unwrap_or_default();
    let args = to_arguments(mrb, args)?;
    let pointers: SmallVec<[_; 4]> = args.iter().map(Variant::var_sys).collect();
    let name = StringName::from(method.as_str());
    let mut receiver = held.0.clone();
    // SAFETY: the name and argument pointers live for the call, which runs on
    // a copy of the value, so the Ruby value never changes.
    let outcome = super::run_engine_call(|answer, error| unsafe {
        sys::interface_fn!(variant_call)(
            receiver.var_sys_mut(),
            name.string_sys(),
            pointers.as_ptr(),
            pointers.len() as i64,
            answer,
            error,
        )
    });
    wrap_answer(mrb, outcome, &type_name(held.0.get_type()), &method, &args)
}

// Godot::Value.__call_static__(name, args): the static method `name` of the
// receiver's type called with `args`, in a one-element array, or nil when
// the type has no such method.
fn call_static(mrb: &Mrb, class: RClass, name: Symbol, args: RArray) -> Result<Value, Error> {
    let kind = kind_by_class(mrb, class)?;
    let method = name.name(mrb).unwrap_or_default();
    let args = to_arguments(mrb, args)?;
    let pointers: SmallVec<[_; 4]> = args.iter().map(Variant::var_sys).collect();
    let name = StringName::from(method.as_str());
    let kind_sys = kind.ord as sys::GDExtensionVariantType;
    // SAFETY: the name and argument pointers live for the call.
    let outcome = super::run_engine_call(|answer, error| unsafe {
        sys::interface_fn!(variant_call_static)(
            kind_sys,
            name.string_sys(),
            pointers.as_ptr(),
            pointers.len() as i64,
            answer,
            error,
        )
    });
    wrap_answer(mrb, outcome, &type_name(kind), &method, &args)
}

// What a call answered, in a one-element array; nil for a method the type
// lacks, and the Godot::CallError GDScript's wording gives any other
// failure.
fn wrap_answer(
    mrb: &Mrb,
    outcome: Result<Variant, sys::GDExtensionCallError>,
    base: &str,
    method: &str,
    args: &[Variant],
) -> Result<Value, Error> {
    match outcome {
        Ok(answer) => Ok(mrb.ary_new_from_values(&[to_ruby(mrb, &answer)]).as_value()),
        Err(error) if error.error == sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD => {
            Ok(qnil().as_value())
        }
        Err(error) => {
            let message = super::object::describe_refusal(&error, method, base, args);
            Err(super::object::call_error(mrb, &message))
        }
    }
}

// Godot::Value.__constant__(name): the receiver's type's constant of that
// name, or nil when it has none.
fn constant(mrb: &Mrb, class: RClass, name: Symbol) -> Result<Value, Error> {
    let kind = kind_by_class(mrb, class)?;
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
        _ => qnil().as_value(),
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
    ToRuby::try_new(answer).map_or_else(|_| qnil().as_value(), |answer| answer.into_value(mrb))
}
