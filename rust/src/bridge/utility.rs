//! The engine's utility functions Ruby lacks, as module functions of
//! `Godot`: game math, the engine's random numbers, and its error reports.
//! Each is one row of a table, so a function is added or left out in one
//! place; Ruby's own `Math`, `rand` and `srand` stay as Ruby's.

use beni::{Error, FromValue, IntoValue, Mrb, RArray, ReprValue, Symbol, Value, value::qnil};
use godot::builtin::{Variant, VariantType};
use godot::global;
use godot::meta::ToGodot;

use super::object::to_arguments;
use super::refusal::{describe_count, make_argument_error, make_type_error};
use super::value::RubyCrossing;

type Utility = fn(&[Variant]) -> Result<Variant, String>;

// Any number of arguments.
const ANY_COUNT: usize = usize::MAX;

/// The utility functions under `Godot`: each name, how many arguments it
/// takes, and the engine function it calls.
static UTILITIES: &[(&str, usize, Utility)] = &[
    ("lerp", 3, |a| Ok(global::lerp(&a[0], &a[1], &a[2]))),
    ("inverse_lerp", 3, |a| {
        Ok(
            global::inverse_lerp(read_float(a, 0)?, read_float(a, 1)?, read_float(a, 2)?)
                .to_variant(),
        )
    }),
    ("remap", 5, |a| {
        let (value, istart, istop) = (read_float(a, 0)?, read_float(a, 1)?, read_float(a, 2)?);
        Ok(global::remap(value, istart, istop, read_float(a, 3)?, read_float(a, 4)?).to_variant())
    }),
    ("move_toward", 3, |a| {
        Ok(
            global::move_toward(read_float(a, 0)?, read_float(a, 1)?, read_float(a, 2)?)
                .to_variant(),
        )
    }),
    ("rotate_toward", 3, |a| {
        Ok(
            global::rotate_toward(read_float(a, 0)?, read_float(a, 1)?, read_float(a, 2)?)
                .to_variant(),
        )
    }),
    ("smoothstep", 3, |a| {
        Ok(
            global::smoothstep(read_float(a, 0)?, read_float(a, 1)?, read_float(a, 2)?)
                .to_variant(),
        )
    }),
    ("ease", 2, |a| {
        Ok(global::ease(read_float(a, 0)?, read_float(a, 1)?).to_variant())
    }),
    ("lerp_angle", 3, |a| {
        Ok(
            global::lerp_angle(read_float(a, 0)?, read_float(a, 1)?, read_float(a, 2)?)
                .to_variant(),
        )
    }),
    ("angle_difference", 2, |a| {
        Ok(global::angle_difference(read_float(a, 0)?, read_float(a, 1)?).to_variant())
    }),
    ("deg_to_rad", 1, |a| {
        Ok(global::deg_to_rad(read_float(a, 0)?).to_variant())
    }),
    ("rad_to_deg", 1, |a| {
        Ok(global::rad_to_deg(read_float(a, 0)?).to_variant())
    }),
    ("wrap", 3, |a| Ok(global::wrap(&a[0], &a[1], &a[2]))),
    ("wrapi", 3, |a| {
        Ok(global::wrapi(read_int(a, 0)?, read_int(a, 1)?, read_int(a, 2)?).to_variant())
    }),
    ("wrapf", 3, |a| {
        Ok(global::wrapf(read_float(a, 0)?, read_float(a, 1)?, read_float(a, 2)?).to_variant())
    }),
    ("snapped", 2, |a| Ok(global::snapped(&a[0], &a[1]))),
    ("pingpong", 2, |a| {
        Ok(global::pingpong(read_float(a, 0)?, read_float(a, 1)?).to_variant())
    }),
    ("clamp", 3, |a| Ok(global::clamp(&a[0], &a[1], &a[2]))),
    ("is_equal_approx", 2, |a| {
        Ok(global::is_equal_approx(read_float(a, 0)?, read_float(a, 1)?).to_variant())
    }),
    ("is_zero_approx", 1, |a| {
        Ok(global::is_zero_approx(read_float(a, 0)?).to_variant())
    }),
    ("floor", 1, |a| Ok(global::floor(&a[0]))),
    ("ceil", 1, |a| Ok(global::ceil(&a[0]))),
    ("round", 1, |a| Ok(global::round(&a[0]))),
    ("floorf", 1, |a| {
        Ok(global::floorf(read_float(a, 0)?).to_variant())
    }),
    ("ceilf", 1, |a| {
        Ok(global::ceilf(read_float(a, 0)?).to_variant())
    }),
    ("roundf", 1, |a| {
        Ok(global::roundf(read_float(a, 0)?).to_variant())
    }),
    ("fposmod", 2, |a| {
        Ok(global::fposmod(read_float(a, 0)?, read_float(a, 1)?).to_variant())
    }),
    ("fmod", 2, |a| {
        Ok(global::fmod(read_float(a, 0)?, read_float(a, 1)?).to_variant())
    }),
    ("linear_to_db", 1, |a| {
        Ok(global::linear_to_db(read_float(a, 0)?).to_variant())
    }),
    ("db_to_linear", 1, |a| {
        Ok(global::db_to_linear(read_float(a, 0)?).to_variant())
    }),
    ("randf", 0, |_| Ok(global::randf().to_variant())),
    ("randi", 0, |_| Ok(global::randi().to_variant())),
    ("randi_range", 2, |a| {
        Ok(global::randi_range(read_int(a, 0)?, read_int(a, 1)?).to_variant())
    }),
    ("randf_range", 2, |a| {
        Ok(global::randf_range(read_float(a, 0)?, read_float(a, 1)?).to_variant())
    }),
    ("randfn", 2, |a| {
        Ok(global::randfn(read_float(a, 0)?, read_float(a, 1)?).to_variant())
    }),
    ("seed", 1, |a| {
        global::seed(read_int(a, 0)?);
        Ok(Variant::nil())
    }),
    ("randomize", 0, |_| {
        global::randomize();
        Ok(Variant::nil())
    }),
    ("rand_from_seed", 1, |a| {
        Ok(global::rand_from_seed(read_int(a, 0)?).to_variant())
    }),
    ("push_error", ANY_COUNT, |a| {
        global::push_error(a);
        Ok(Variant::nil())
    }),
    ("push_warning", ANY_COUNT, |a| {
        global::push_warning(a);
        Ok(Variant::nil())
    }),
    ("is_same", 2, |a| {
        Ok(global::is_same(&a[0], &a[1]).to_variant())
    }),
];

// The argument at `index` as a float, converted as the engine converts one.
fn read_float(args: &[Variant], index: usize) -> Result<f64, String> {
    args[index]
        .try_to_relaxed::<f64>()
        .map_err(|_| describe_conversion_failure(&args[index], index, VariantType::FLOAT))
}

// The argument at `index` as an integer, converted as the engine converts one.
fn read_int(args: &[Variant], index: usize) -> Result<i64, String> {
    args[index]
        .try_to_relaxed::<i64>()
        .map_err(|_| describe_conversion_failure(&args[index], index, VariantType::INT))
}

fn describe_conversion_failure(arg: &Variant, index: usize, to: VariantType) -> String {
    let name = |kind: VariantType| global::type_string(i64::from(kind.ord)).to_string();
    format!(
        "Cannot convert argument {} from {} to {}.",
        index + 1,
        name(arg.get_type()),
        name(to)
    )
}

/// Each utility function's name, how many arguments it takes, nil for any
/// number, and the row a call names it by, for `Godot` to define.
pub fn utilities(mrb: &Mrb, _godot: Value) -> Value {
    let rows: Vec<Value> = UTILITIES
        .iter()
        .enumerate()
        .filter_map(|(row, (name, arity, _))| {
            let name = Symbol::from(mrb.intern(name.as_bytes()).ok()?).as_value();
            let arity = if *arity == ANY_COUNT {
                qnil().as_value()
            } else {
                (*arity as i64).into_value(mrb)
            };
            let row = (row as i64).into_value(mrb);
            Some(mrb.ary_new_from_values(&[name, arity, row]).as_value())
        })
        .collect();
    mrb.ary_new_from_values(&rows).as_value()
}

/// Godot.__call_utility__(row, *args): the utility function in that row called
/// with the arguments given.
pub fn call_utility(mrb: &Mrb, _godot: Value, args: &[Value]) -> Result<Value, Error> {
    let Some((&row, args)) = args.split_first() else {
        return Err(make_argument_error(mrb, &describe_count(0, 1)));
    };
    let Some(row) = i64::from_value(row) else {
        return Ok(qnil().as_value());
    };
    call_row(mrb, row, args.iter().copied())
}

/// Godot.__apply_utility__(row, args): the utility function in that row
/// called with the arguments in `args`, for one taking any number.
pub fn apply_utility(mrb: &Mrb, _godot: Value, row: i64, args: RArray) -> Result<Value, Error> {
    call_row(mrb, row, args.entries(mrb))
}

// The utility function in `row` called with `args`, or nil when no row is
// that one.
fn call_row(
    mrb: &Mrb,
    row: i64,
    args: impl ExactSizeIterator<Item = Value>,
) -> Result<Value, Error> {
    let Some((name, arity, utility)) = usize::try_from(row).ok().and_then(|row| UTILITIES.get(row))
    else {
        return Ok(qnil().as_value());
    };
    if *arity != ANY_COUNT && args.len() != *arity {
        return Err(make_argument_error(mrb, &describe_count(args.len(), arity)));
    }
    let args = to_arguments(mrb, super::data(mrb), args)?;
    let answer = utility(&args).map_err(|reason| {
        make_type_error(
            mrb,
            &format!("Invalid type in utility function '{name}'. {reason}"),
        )
    })?;
    Ok(RubyCrossing(&answer).into_value(mrb))
}
