//! How a call the engine or Ruby refused is told to Ruby: the error each
//! refusal raises, worded as mruby or GDScript words it.

use std::ffi::CStr;
use std::fmt;

use beni::{Error, ExceptionClass, FromValue, Module as _, Mrb, ReprValue};
use godot::builtin::{StringName, Variant, VariantType};
use godot::global::type_string;
use godot::meta::error::CallError;
use godot::obj::EngineEnum;
use godot::sys;

use super::bound_method::Arity;
use crate::hint::name_type;

// The error a call the engine refused raises, as `make_refusal_error` sorts it.
// gdext hands the engine's reason only as text, so the expected count, the
// argument and the types are read back from it; the count names those the
// method requires, as the class database lists them.
pub(super) fn make_call_refusal(
    mrb: &Mrb,
    error: &CallError,
    base: &str,
    method: &str,
    given: usize,
) -> Error {
    let reason = error.message(false);
    let reason = reason.rsplit("Reason: ").next().unwrap_or_default();
    if let Some(expected) = parse_parameter_count(reason) {
        let message = Arity::find(base, method).map_or_else(
            || describe_count(given, expected),
            |arity| describe_count(given, arity.required),
        );
        make_argument_error(mrb, &message)
    } else if let Some((argument, from, to)) = parse_conversion(reason) {
        make_type_error(
            mrb,
            &describe_type_refusal(method, base, argument, &from, &to),
        )
    } else {
        make_call_error(
            mrb,
            &format!("Invalid call to function '{method}' in base '{base}': {reason}"),
        )
    }
}

/// The error a call of `method` on `base` with `args` the engine refused
/// with `error` raises: ArgumentError for the number of arguments, worded as
/// mruby words it, naming those the method's `arity` requires when it is
/// known; TypeError for an argument's type, worded as GDScript's
/// untyped call; Godot::CallError for what Ruby has no error for.
pub(super) fn make_refusal_error(
    mrb: &Mrb,
    error: &sys::GDExtensionCallError,
    method: &str,
    base: &str,
    args: &[Variant],
    arity: Option<Arity>,
) -> Error {
    match error.error {
        sys::GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT => {
            let index = error.argument as usize;
            let from = args
                .get(index)
                .map_or_else(String::new, |arg| name_type(arg.get_type()));
            let to = name_type(<VariantType as EngineEnum>::from_ord(error.expected));
            make_type_error(
                mrb,
                &describe_type_refusal(method, base, &(index + 1).to_string(), &from, &to),
            )
        }
        sys::GDEXTENSION_CALL_ERROR_TOO_MANY_ARGUMENTS
        | sys::GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS => {
            let message = match arity {
                Some(arity) if arity.is_vararg && args.len() >= arity.required => {
                    describe_forwarded_count(error, base, args, arity)
                }
                Some(arity) => describe_count(args.len(), arity.required),
                None => describe_count(args.len(), error.expected),
            };
            make_argument_error(mrb, &message)
        }
        _ => make_call_error(
            mrb,
            &format!("Invalid call to function '{method}' in base '{base}'."),
        ),
    }
}

// Ruby's words for a count the engine refused past a method taking any
// number, which `Arity::check` let through: the method forwarded the
// arguments past those it requires to the method its first one names, as
// `call` does, so the words name that method's count, as Ruby's `send` does.
fn describe_forwarded_count(
    error: &sys::GDExtensionCallError,
    base: &str,
    args: &[Variant],
    arity: Arity,
) -> String {
    let forwarded = args.len() - arity.required;
    let required = args
        .first()
        .and_then(|name| name.try_to::<StringName>().ok())
        .and_then(|name| Arity::find(base, &name.to_string()))
        .map_or(error.expected as usize, |method| method.required);
    describe_count(forwarded, required)
}

/// Ruby's words for a call given `given` arguments where `expected` are taken.
pub(super) fn describe_count(given: usize, expected: impl fmt::Display) -> String {
    format!("wrong number of arguments (given {given}, expected {expected})")
}

// GDScript's words for a call given an argument of a type it cannot take.
fn describe_type_refusal(method: &str, base: &str, argument: &str, from: &str, to: &str) -> String {
    format!(
        "Invalid type in function '{method}' in base '{base}'. \
         Cannot convert argument {argument} from {from} to {to}."
    )
}

// "function has N parameters, but received M arguments": N.
fn parse_parameter_count(reason: &str) -> Option<&str> {
    reason
        .strip_prefix("function has ")?
        .split_once(" parameter")
        .map(|(count, _)| count)
}

// "parameter #I -- cannot convert from FROM to TO": I and the two types as
// GDScript names them.
fn parse_conversion(reason: &str) -> Option<(&str, String, String)> {
    let (argument, types) = reason
        .strip_prefix("parameter #")?
        .split_once(" -- cannot convert from ")?;
    let (from, to) = types.split_once(" to ")?;
    Some((argument, type_name_by_debug(from)?, type_name_by_debug(to)?))
}

// The name GDScript gives the variant type gdext debug-prints as `debug`.
fn type_name_by_debug(debug: &str) -> Option<String> {
    (0..VariantType::MAX.ord)
        .map(<VariantType as EngineEnum>::from_ord)
        .find(|kind| format!("{kind:?}") == debug)
        .map(|kind| type_string(i64::from(kind.ord)).to_string())
}

pub(super) fn make_type_error(mrb: &Mrb, message: &str) -> Error {
    make_error(mrb, c"TypeError", message)
}

pub(super) fn make_zero_division_error(mrb: &Mrb, message: &str) -> Error {
    make_error(mrb, c"ZeroDivisionError", message)
}

pub(super) fn make_argument_error(mrb: &Mrb, message: &str) -> Error {
    make_error(mrb, c"ArgumentError", message)
}

pub(super) fn make_not_implemented_error(mrb: &Mrb, message: &str) -> Error {
    make_error(mrb, c"NotImplementedError", message)
}

// An error of Ruby's own `class` carrying `message`.
fn make_error(mrb: &Mrb, class: &CStr, message: &str) -> Error {
    match mrb.exc_get(class) {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}

pub(super) fn make_call_error(mrb: &Mrb, message: &str) -> Error {
    let class = mrb
        .module_get(c"Godot")
        .and_then(|godot| godot.class_get(mrb, c"CallError"));
    match class.map(|class| ExceptionClass::from_value(class.as_value())) {
        Ok(Some(class)) => Error::new(mrb, class, message),
        Ok(None) => unreachable!("Godot::CallError is an exception class"),
        Err(error) => error,
    }
}
