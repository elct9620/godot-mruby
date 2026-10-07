//! The engine's value types in Ruby: `Vector2`, `Color`, `NodePath` and the
//! rest are classes under `Godot` descending from `Godot::Value`, whose
//! values are built, read and computed with by the engine itself, and never
//! change. A Vector2, held as its components, goes to the engine's typed
//! operators, which take it as it is; every other type, and whatever those
//! do not answer, goes to the engine's variant calls, by name.

use std::borrow::Cow;
use std::ptr;
use std::sync::OnceLock;

use beni::{
    DataType, Error, FromValue, IntoValue, Module, Mrb, Object as _, RArray, RClass, RModule,
    ReprValue, Symbol, TryConvert, TypedData, Value, method, value::qnil,
};
use godot::builtin::{GString, StringName, Variant, VariantOperator, VariantType, Vector2, real};
use godot::meta::ToGodot;
use godot::obj::EngineEnum;
use godot::sys;
use smallvec::SmallVec;

use super::BridgeData;
use super::bound_member::BoundMember;
use super::object::{name_by_symbol, to_arguments, type_error, zero_division_error};
use super::value::{self, ToRuby};
use crate::hint::type_name;

/// A value of one of the engine's value types, as Ruby holds it: a Vector2,
/// which games build and compute with most, as its components, so it
/// reaches the engine without a variant, and any other in the engine's
/// variant.
#[derive(Clone)]
pub enum EngineValue {
    Held(Variant),
    Vector2(Vector2),
}

// SAFETY: a value type holds no object and no reference into the engine
// that another thread could change, and the realm holding it is entered by
// one thread at a time.
unsafe impl Send for EngineValue {}

static ENGINE_VALUE: DataType<EngineValue> = DataType::new(c"Godot::Value");

// SAFETY: `define` marks Godot::Value, and every value type's class under
// Godot descends from it.
unsafe impl TypedData for EngineValue {
    fn class(mrb: &Mrb) -> RClass {
        value_class(mrb, super::data(mrb))
    }

    fn data_type() -> &'static DataType<Self> {
        &ENGINE_VALUE
    }
}

/// Godot::Value, found once for a realm and kept in its bridge data.
pub(super) fn value_class(mrb: &Mrb, data: &BridgeData) -> RClass {
    super::find_class_once(mrb, &data.value_class, || {
        mrb.module_get(c"Godot")
            .and_then(|godot| godot.class_get(mrb, c"Value"))
            .expect("the Godot gem defines Godot::Value")
    })
}

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
    class.undef_default_alloc_func(mrb);
    class.define_singleton_method(mrb, c"__has_value_type__", method!(has_value_type, 1))?;
    class.define_singleton_method(mrb, c"new", method!(construct, -1))?;
    class.define_singleton_method(mrb, c"__call_static__", method!(call_static, 2))?;
    class.define_singleton_method(mrb, c"__constant__", method!(constant, 1))?;
    class.define_singleton_method(mrb, c"__engine_names__", method!(engine_names, 0))?;
    class.define_private_method(mrb, c"__resolve__", method!(resolve, 1))?;
    class.define_private_method(mrb, c"__member__", method!(member, 1))?;
    class.define_private_method(mrb, c"__get__", method!(get, 1))?;
    class.define_private_method(mrb, c"__call__", method!(call, 2))?;
    class.define_private_method(mrb, c"__hash__", method!(hash, 0))?;
    class.define_private_method(mrb, c"__copy_value__", method!(copy_value, 0))?;
    class.define_method(mrb, c"+", method!(add, 1))?;
    class.define_method(mrb, c"-", method!(subtract, 1))?;
    class.define_method(mrb, c"*", method!(multiply, 1))?;
    class.define_method(mrb, c"/", method!(divide, 1))?;
    class.define_method(mrb, c"%", method!(modulo, 1))?;
    class.define_method(mrb, c"**", method!(power, 1))?;
    class.define_method(mrb, c"<", method!(less, 1))?;
    class.define_method(mrb, c"<=", method!(less_equal, 1))?;
    class.define_method(mrb, c">", method!(greater, 1))?;
    class.define_method(mrb, c">=", method!(greater_equal, 1))?;
    class.define_method(mrb, c"==", method!(is_equal, 1))?;
    class.define_method(mrb, c"-@", method!(negate, 0))?;
    class.define_method(mrb, c"+@", method!(keep_sign, 0))?;
    class.define_method(mrb, c"to_s", method!(to_s, 0))?;
    Ok(())
}

/// The Ruby value for a value of a value type: a value of its class under
/// Godot.
pub(super) fn ruby_value(mrb: &Mrb, data: &BridgeData, variant: &Variant) -> Value {
    let held = match variant.get_type() {
        VariantType::VECTOR2 => EngineValue::Vector2(variant.to()),
        _ => EngineValue::Held(variant.clone()),
    };
    wrap(mrb, data, held)
}

// The Ruby value holding `held`: a value of its type's class under Godot.
fn wrap(mrb: &Mrb, data: &BridgeData, held: EngineValue) -> Value {
    match class_by_kind(mrb, data, held.kind()) {
        Some(class) => mrb.wrap_as(held, class).as_value(),
        None => qnil().as_value(),
    }
}

// The class under Godot of the value type `kind`, found once for a realm and
// kept for it, rooted for the collector so the class lives while it is kept.
fn class_by_kind(mrb: &Mrb, data: &BridgeData, kind: VariantType) -> Option<RClass> {
    let classes = &data.value_classes;
    let index = kind.ord as usize;
    if let Some(class) = classes.borrow().get(index).copied().flatten() {
        return Some(class);
    }
    let class = mrb
        .module_get(c"Godot")
        .and_then(|godot| godot.class_get(mrb, type_name(kind).as_str()))
        .ok()?;
    keep_class(mrb, data, kind, class);
    Some(class)
}

// Keeps `class` as the class of the value type `kind`, rooted for the
// collector so it lives while it is kept.
fn keep_class(mrb: &Mrb, data: &BridgeData, kind: VariantType, class: RClass) {
    mrb.gc_register_forever(class.as_value());
    let mut classes = data.value_classes.borrow_mut();
    let index = kind.ord as usize;
    if classes.len() <= index {
        classes.resize(index + 1, None);
    }
    classes[index] = Some(class);
}

impl EngineValue {
    /// The value as the engine takes it.
    pub fn variant(&self) -> Variant {
        self.as_variant().into_owned()
    }

    // The value in a variant, borrowed when the engine's variant holds it.
    fn as_variant(&self) -> Cow<'_, Variant> {
        match self {
            Self::Held(variant) => Cow::Borrowed(variant),
            Self::Vector2(vector) => Cow::Owned(vector.to_variant()),
        }
    }

    fn kind(&self) -> VariantType {
        match self {
            Self::Held(variant) => variant.get_type(),
            Self::Vector2(_) => VariantType::VECTOR2,
        }
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

// The value type a class under Godot stands for: found among the kept
// classes by identity, and by the class's name only before it is kept. A
// class Godot names now is kept for its type in place of one it named
// before, so the type's values are made of the class Ruby names.
fn kind_by_class(mrb: &Mrb, data: &BridgeData, class: RClass) -> Result<VariantType, Error> {
    let kept =
        data.value_classes.borrow().iter().position(|kept| {
            kept.is_some_and(|kept| kept.as_value().is_equal(mrb, class.as_value()))
        });
    if let Some(ord) = kept {
        return Ok(<VariantType as EngineEnum>::from_ord(ord as i32));
    }
    let path = class.path(mrb).unwrap_or_default();
    let name = path.strip_prefix("Godot::").unwrap_or(&path);
    let kind = kind_by_name(name)
        .ok_or_else(|| type_error(mrb, &format!("{path} is no value type of the engine")))?;
    if is_named_by_godot(mrb, class, name) {
        keep_class(mrb, data, kind, class);
    }
    Ok(kind)
}

// Whether `class` is what Godot's constant `name` holds now.
fn is_named_by_godot(mrb: &Mrb, class: RClass, name: &str) -> bool {
    mrb.module_get(c"Godot")
        .ok()
        .filter(|godot| godot.const_defined_at(mrb, name))
        .and_then(|godot| godot.const_get::<_, RClass>(mrb, name).ok())
        .is_some_and(|named| named.as_value().is_equal(mrb, class.as_value()))
}

// Godot::Value.new(*args): the value the engine's constructor of the
// receiver's type that takes `args` builds.
fn construct(mrb: &Mrb, class: RClass, args: &[Value]) -> Result<Value, Error> {
    let data = super::data(mrb);
    let kind = kind_by_class(mrb, data, class)?;
    if kind == VariantType::VECTOR2
        && let Some(vector) = read_vector2(args)
    {
        return Ok(wrap(mrb, data, EngineValue::Vector2(vector)));
    }
    let args = to_arguments(mrb, data, args.iter().copied())?;
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
        Ok(built) => Ok(ruby_value(mrb, data, &built)),
        Err(_) => {
            let message = format!(
                "Invalid call. Nonexistent '{}' constructor.",
                type_name(kind)
            );
            Err(super::object::argument_error(mrb, &message))
        }
    }
}

// The Vector2 the engine's constructor taking two numbers builds from
// `args`, when they are two numbers: each read as the engine reads a float
// argument, then narrowed to the engine's real.
fn read_vector2(args: &[Value]) -> Option<Vector2> {
    let component = |arg: Value| {
        i64::from_value(arg)
            .map(|integer| integer as f64)
            .or_else(|| f64::from_value(arg))
            .map(|float| float as real)
    };
    match args {
        [x, y] => Some(Vector2::new(component(*x)?, component(*y)?)),
        _ => None,
    }
}

// A value type's name, its members, and its methods with the hash of each
// one's signature.
type ValueNames = (
    &'static str,
    &'static [&'static str],
    &'static [(&'static str, i64)],
);

// Each value type's names, as the API the extension is built against lists
// them.
const VALUE_NAMES: &[ValueNames] = include!(concat!(env!("OUT_DIR"), "/value_names.rs"));

// Godot::Value.__engine_names__: the names of the members and methods of the
// receiver's value type that the running engine has.
fn engine_names(mrb: &Mrb, class: RClass) -> Result<Value, Error> {
    let kind = kind_by_class(mrb, super::data(mrb), class)?;
    let kind_name = type_name(kind);
    let kind = kind.ord as sys::GDExtensionVariantType;
    let Some((_, members, methods)) = VALUE_NAMES.iter().find(|(name, ..)| *name == kind_name)
    else {
        return Ok(mrb.ary_new_from_values(&[]).as_value());
    };
    // SAFETY: the interface is initialized while the extension runs, and
    // each name lives for its call.
    let has_member = |name: &str| unsafe {
        sys::interface_fn!(variant_has_member)(kind, StringName::from(name).string_sys()) != 0
    };
    let has_method = |name: &str, hash: i64| unsafe {
        sys::interface_fn!(variant_get_ptr_builtin_method)(
            kind,
            StringName::from(name).string_sys(),
            hash,
        )
        .is_some()
    };
    let names = members
        .iter()
        .copied()
        .filter(|name| has_member(name))
        .chain(
            methods
                .iter()
                .filter(|(name, hash)| has_method(name, *hash))
                .map(|(name, _)| *name),
        )
        .map(|name| {
            mrb.intern(name.as_bytes())
                .map(|id| Symbol::from(id).as_value())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(mrb.ary_new_from_values(&names).as_value())
}

// Godot::Value#__resolve__(name): [:member, bound] when the value's type
// has a member of that name, with the member bound through the engine's
// getter, or nil in its place when there is none; :method when the type
// has a method of that name; or nil.
fn resolve(mrb: &Mrb, held: &EngineValue, name: Symbol) -> Result<Value, Error> {
    let name = name_by_symbol(mrb, name);
    let value = held.as_variant();
    let kind = value.get_type().ord as sys::GDExtensionVariantType;
    // SAFETY: the interface is initialized while the extension runs, and the
    // name lives for the call.
    if unsafe { sys::interface_fn!(variant_has_member)(kind, name.string_sys()) } != 0 {
        let member = Symbol::from(mrb.intern(b"member")?).as_value();
        let bound = read_named(&value, &name)
            .and_then(|read| BoundMember::find(&value, &name, &read))
            .map_or_else(|| qnil().as_value(), |bound| mrb.wrap(bound).as_value());
        return Ok(mrb.ary_new_from_values(&[member, bound]).as_value());
    }
    // SAFETY: the interface is initialized while the extension runs, and the
    // value and the name live for the call.
    if unsafe { sys::interface_fn!(variant_has_method)(value.var_sys(), name.string_sys()) } != 0 {
        return Ok(Symbol::from(mrb.intern(b"method")?).as_value());
    }
    Ok(qnil().as_value())
}

// Godot::Value#__member__(name): the member of that name, which the value's
// type has.
fn member(mrb: &Mrb, held: &EngineValue, name: Symbol) -> Value {
    let name = name_by_symbol(mrb, name);
    read_named(&held.as_variant(), &name)
        .map_or_else(|| qnil().as_value(), |found| to_ruby(mrb, &found))
}

// Godot::Value#__get__(bound): the member `bound` reads, through the
// engine's getter, or from the components a Vector2 is held as.
fn get(mrb: &Mrb, held: &EngineValue, bound: &BoundMember) -> Value {
    if let EngineValue::Vector2(vector) = held
        && let Some(component) = bound.read_component(*vector)
    {
        return f64::from(component).into_value(mrb);
    }
    bound
        .read(&held.as_variant())
        .map_or_else(|| qnil().as_value(), |found| to_ruby(mrb, &found))
}

// The member `name` of `value`, as the engine reads it by name.
fn read_named(value: &Variant, name: &StringName) -> Option<Variant> {
    // SAFETY: the interface is initialized while the extension runs, and
    // `variant_get_named` writes the member when it reports it valid.
    unsafe {
        Variant::new_with_var_uninit_result(|found| {
            let mut valid = false as sys::GDExtensionBool;
            sys::interface_fn!(variant_get_named)(
                value.var_sys(),
                name.string_sys(),
                found,
                ptr::addr_of_mut!(valid),
            );
            if valid == 0 { Err(()) } else { Ok(()) }
        })
    }
    .ok()
}

// Godot::Value#__call__(name, args): what the engine method `name`, which
// the value's type has, answers for `args`.
fn call(mrb: &Mrb, held: &EngineValue, name: Symbol, args: RArray) -> Result<Value, Error> {
    let name = name_by_symbol(mrb, name);
    let args = to_arguments(mrb, super::data(mrb), args.entries(mrb))?;
    let pointers: SmallVec<[_; 4]> = args.iter().map(Variant::var_sys).collect();
    let mut receiver = held.variant();
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
    take_answer(mrb, outcome, held.kind(), &name, &args)
}

// Godot::Value.__call_static__(name, args): the static method `name` of the
// receiver's type called with `args`, in a one-element array, or nil when
// the type has no such method.
fn call_static(mrb: &Mrb, class: RClass, name: Symbol, args: RArray) -> Result<Value, Error> {
    let kind = kind_by_class(mrb, super::data(mrb), class)?;
    let name = name_by_symbol(mrb, name);
    let args = to_arguments(mrb, super::data(mrb), args.entries(mrb))?;
    let pointers: SmallVec<[_; 4]> = args.iter().map(Variant::var_sys).collect();
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
    match outcome {
        Err(error) if error.error == sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD => {
            Ok(qnil().as_value())
        }
        outcome => {
            let answer = take_answer(mrb, outcome, kind, &name, &args)?;
            Ok(mrb.ary_new_from_values(&[answer]).as_value())
        }
    }
}

// What a call of `method` on a value of type `kind` answered, or the error
// its refusal raises.
fn take_answer(
    mrb: &Mrb,
    outcome: Result<Variant, sys::GDExtensionCallError>,
    kind: VariantType,
    method: &StringName,
    args: &[Variant],
) -> Result<Value, Error> {
    outcome
        .map(|answer| to_ruby(mrb, &answer))
        .map_err(|error| {
            super::object::refusal_error(
                mrb,
                &error,
                &method.to_string(),
                &type_name(kind),
                args,
                None,
            )
        })
}

// Godot::Value.__constant__(name): the receiver's type's constant of that
// name, or nil when it has none.
fn constant(mrb: &Mrb, class: RClass, name: Symbol) -> Result<Value, Error> {
    let kind = kind_by_class(mrb, super::data(mrb), class)?;
    let name = name_by_symbol(mrb, name);
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

// Each of the engine's operators a value answers, as Godot::Value's method
// of that operator's Ruby name: what the operator answers for the value and
// the other operand.
macro_rules! operators {
    ($($method:ident: $op:ident, $shown:literal, $typed:literal;)*) => {$(
        fn $method(mrb: &Mrb, held: &EngineValue, other: Value) -> Result<Value, Error> {
            let data = super::data(mrb);
            if $typed
                && let EngineValue::Vector2(vector) = held
                && let Some(answer) = read_operand(mrb, data, other).and_then(|operand| {
                    operate_typed(mrb, data, *vector, VariantOperator::$op, operand)
                })
            {
                return Ok(answer);
            }
            let other =
                value::to_engine(mrb, data, other, 1).map_err(|reason| type_error(mrb, &reason))?;
            operate(mrb, held, VariantOperator::$op, $shown, &other)
        }
    )*};
}

// Division, modulo and power stay with the variant call, where the engine
// refuses a division by zero for the types that forbid it; a typed function
// never checks.
operators! {
    add: ADD, "+", true;
    subtract: SUBTRACT, "-", true;
    multiply: MULTIPLY, "*", true;
    divide: DIVIDE, "/", false;
    modulo: MODULO, "%", false;
    power: POWER, "**", false;
    less: LESS, "<", true;
    less_equal: LESS_EQUAL, "<=", true;
    greater: GREATER, ">", true;
    greater_equal: GREATER_EQUAL, ">=", true;
}

// Godot::Value#==: whether `other` is a value the engine's equality finds
// equal to this one; any other object is not.
fn is_equal(mrb: &Mrb, held: &EngineValue, other: Value) -> Result<Value, Error> {
    let data = super::data(mrb);
    if !other.is_kind_of(mrb, value_class(mrb, data)) {
        return Ok(false.into_value(mrb));
    }
    let other = <&EngineValue>::try_convert(other, mrb)?;
    if let (EngineValue::Vector2(vector), EngineValue::Vector2(right)) = (held, other)
        && let Some(answer) = operate_typed(
            mrb,
            data,
            *vector,
            VariantOperator::EQUAL,
            Operand::Vector2(*right),
        )
    {
        return Ok(answer);
    }
    operate(mrb, held, VariantOperator::EQUAL, "==", &other.as_variant())
}

// Godot::Value#-@: the engine's negation of the value.
fn negate(mrb: &Mrb, held: &EngineValue) -> Result<Value, Error> {
    operate_unary(mrb, held, VariantOperator::NEGATE, "unary-")
}

// Godot::Value#+@: the engine's unary plus of the value.
fn keep_sign(mrb: &Mrb, held: &EngineValue) -> Result<Value, Error> {
    operate_unary(mrb, held, VariantOperator::POSITIVE, "unary+")
}

// What the engine's unary operator `op` answers for the value.
fn operate_unary(
    mrb: &Mrb,
    held: &EngineValue,
    op: VariantOperator,
    shown: &str,
) -> Result<Value, Error> {
    if let EngineValue::Vector2(vector) = held
        && let Some(answer) = operate_typed(mrb, super::data(mrb), *vector, op, Operand::None)
    {
        return Ok(answer);
    }
    operate(mrb, held, op, shown, &Variant::nil())
}

// The right operand of a typed operator on a Vector2: one the engine's typed
// functions take as Ruby holds it, or none for a unary operator.
#[derive(Clone, Copy)]
enum Operand {
    Vector2(Vector2),
    Int(i64),
    Float(f64),
    None,
}

impl Operand {
    fn kind(&self) -> VariantType {
        match self {
            Self::Vector2(_) => VariantType::VECTOR2,
            Self::Int(_) => VariantType::INT,
            Self::Float(_) => VariantType::FLOAT,
            Self::None => VariantType::NIL,
        }
    }

    // The operand where a typed function reads it; a unary operator reads none.
    fn as_ptr(&self) -> sys::GDExtensionConstTypePtr {
        match self {
            Self::Vector2(vector) => ptr::from_ref(vector).cast(),
            Self::Int(integer) => ptr::from_ref(integer).cast(),
            Self::Float(float) => ptr::from_ref(float).cast(),
            Self::None => ptr::null(),
        }
    }
}

// `other` as the right operand of a typed operator, when it is one.
fn read_operand(mrb: &Mrb, data: &BridgeData, other: Value) -> Option<Operand> {
    if let Some(integer) = i64::from_value(other) {
        return Some(Operand::Int(integer));
    }
    if let Some(float) = f64::from_value(other) {
        return Some(Operand::Float(float));
    }
    if other.is_kind_of(mrb, value_class(mrb, data))
        && let Ok(EngineValue::Vector2(vector)) = <&EngineValue>::try_convert(other, mrb)
    {
        return Some(Operand::Vector2(*vector));
    }
    None
}

type Evaluator = unsafe extern "C" fn(
    sys::GDExtensionConstTypePtr,
    sys::GDExtensionConstTypePtr,
    sys::GDExtensionTypePtr,
);

const OPERATORS: usize = sys::GDEXTENSION_VARIANT_OP_MAX as usize;
const OPERAND_KINDS: usize = VariantType::VECTOR2.ord as usize + 1;

// The engine's typed function for `op` on a Vector2 and an operand of
// `right`, found once: the engine's functions stay put while it runs. A
// function is kept only once the engine's variant call, given values of the
// same types, answers the type `answer_kind` says it writes.
fn evaluator_by_operands(op: VariantOperator, right: VariantType) -> Option<Evaluator> {
    static EVALUATORS: [[OnceLock<Option<Evaluator>>; OPERAND_KINDS]; OPERATORS] =
        [const { [const { OnceLock::new() }; OPERAND_KINDS] }; OPERATORS];
    *EVALUATORS[op.ord() as usize][right.ord as usize].get_or_init(|| {
        // SAFETY: the interface is initialized while the extension runs.
        let evaluate = unsafe {
            sys::interface_fn!(variant_get_ptr_operator_evaluator)(
                op.ord() as sys::GDExtensionVariantOperator,
                sys::GDEXTENSION_VARIANT_TYPE_VECTOR2,
                right.ord as sys::GDExtensionVariantType,
            )
        }?;
        let left = new_default(VariantType::VECTOR2);
        let (answer, valid) = evaluate_variants(op, &left, &new_default(right));
        (valid && answer.get_type() == answer_kind(op)).then_some(evaluate)
    })
}

// The type a typed operator on a Vector2 writes: a bool for a comparison,
// else a Vector2, the only answer the engine's arithmetic on one gives.
fn answer_kind(op: VariantOperator) -> VariantType {
    match op {
        VariantOperator::EQUAL
        | VariantOperator::LESS
        | VariantOperator::LESS_EQUAL
        | VariantOperator::GREATER
        | VariantOperator::GREATER_EQUAL => VariantType::BOOL,
        _ => VariantType::VECTOR2,
    }
}

// What the engine's typed operator `op` answers for `vector` and `right`, or
// none when the engine has no typed function for the two.
fn operate_typed(
    mrb: &Mrb,
    data: &BridgeData,
    vector: Vector2,
    op: VariantOperator,
    right: Operand,
) -> Option<Value> {
    let evaluate = evaluator_by_operands(op, right.kind())?;
    let left = ptr::from_ref(&vector).cast();
    // SAFETY: each operand is of the type the function was found for, and
    // the answer is of the type `answer_kind` names, which the engine was
    // seen to answer before the function was kept.
    unsafe {
        if answer_kind(op) == VariantType::BOOL {
            let mut answer = false;
            evaluate(left, right.as_ptr(), ptr::from_mut(&mut answer).cast());
            Some(answer.into_value(mrb))
        } else {
            let mut answer = Vector2::ZERO;
            evaluate(left, right.as_ptr(), ptr::from_mut(&mut answer).cast());
            Some(wrap(mrb, data, EngineValue::Vector2(answer)))
        }
    }
}

// What the engine's operator `op` answers for the value and `other`. A
// refusal raises as GDScript words it: with the reason the engine gives,
// which for a value's operator is a division or modulo by zero, or else as
// the TypeError of an operator the engine lacks for them.
fn operate(
    mrb: &Mrb,
    held: &EngineValue,
    op: VariantOperator,
    shown: &str,
    other: &Variant,
) -> Result<Value, Error> {
    let (answer, valid) = evaluate_variants(op, &held.as_variant(), other);
    if valid {
        return Ok(to_ruby(mrb, &answer));
    }
    if answer.get_type() == VariantType::STRING {
        let message = format!("{answer} in operator '{shown}'.");
        return Err(zero_division_error(mrb, &message));
    }
    let message = format!(
        "Invalid operands '{}' and '{}' in operator '{shown}'.",
        type_name(held.kind()),
        type_name(other.get_type())
    );
    Err(type_error(mrb, &message))
}

// A value of type `kind` as the engine's default constructor builds it: one
// a typed function writes over, or an operand to try an operator on.
pub(super) fn new_default(kind: VariantType) -> Variant {
    // SAFETY: the interface is initialized while the extension runs, and
    // every type has a constructor taking nothing.
    unsafe {
        Variant::new_with_var_uninit(|answer| {
            let mut error = sys::default_call_error();
            sys::interface_fn!(variant_construct)(
                kind.ord as sys::GDExtensionVariantType,
                answer,
                ptr::null(),
                0,
                ptr::addr_of_mut!(error),
            );
        })
    }
}

// What the engine's variant call answers for `op` on `left` and `right`, and
// whether the engine took the operands; a refusal answers its reason.
fn evaluate_variants(op: VariantOperator, left: &Variant, right: &Variant) -> (Variant, bool) {
    let mut valid = false as sys::GDExtensionBool;
    // SAFETY: the operands live for the call, and the engine initializes the
    // answer before writing either it or its reason for refusing.
    let answer = unsafe {
        Variant::new_with_var_uninit(|answer| {
            sys::interface_fn!(variant_evaluate)(
                op.ord() as sys::GDExtensionVariantOperator,
                left.var_sys(),
                right.var_sys(),
                answer,
                ptr::addr_of_mut!(valid),
            )
        })
    };
    (answer, valid != 0)
}

// Godot::Value#__hash__: the engine's hash of the value, which equal values
// share.
// Godot::Value#__copy_value__: a new value of the receiver's class holding
// a copy of the receiver's, as the engine copies a value.
fn copy_value(mrb: &Mrb, held: &EngineValue) -> Value {
    wrap(mrb, super::data(mrb), held.clone())
}

fn hash(_mrb: &Mrb, held: &EngineValue) -> i64 {
    // SAFETY: the interface is initialized while the extension runs, and
    // the value lives for the call.
    unsafe { sys::interface_fn!(variant_hash)(held.as_variant().var_sys()) }
}

// Godot::Value#to_s: the value as the engine prints it.
fn to_s(mrb: &Mrb, held: &EngineValue) -> Value {
    let printed = GString::from(&held.as_variant().to_string()).to_string();
    mrb.str_new(printed.as_bytes()).as_value()
}

// What the engine answered for a value, as Ruby is given it; a value type's
// answers hold no container Ruby could not take.
fn to_ruby(mrb: &Mrb, answer: &Variant) -> Value {
    ToRuby::try_new(answer).map_or_else(|_| qnil().as_value(), |answer| answer.into_value(mrb))
}
