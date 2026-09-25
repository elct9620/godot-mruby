//! The engine's utility functions Ruby lacks, as module functions of
//! `Godot`: game math, the engine's random numbers, and its error reports.
//! Each is one row of a table, so a function is added or left out in one
//! place; Ruby's own `Math`, `rand` and `srand` stay as Ruby's.

use beni::{Array, Error, IntoValue, Mrb, ReprValue, Symbol, Value};
use godot::builtin::{Variant, VariantType};
use godot::global;
use godot::meta::ToGodot;

use super::object::call_error;
use super::value::{self, ToRuby};

type Utility = fn(&[Variant]) -> Result<Variant, String>;

// Any number of arguments.
const ANY: usize = usize::MAX;

/// The utility functions under `Godot`: each name, how many arguments it
/// takes, and the engine function it calls.
static UTILITIES: &[(&str, usize, Utility)] = &[
    ("lerp", 3, |a| Ok(global::lerp(&a[0], &a[1], &a[2]))),
    ("inverse_lerp", 3, |a| {
        Ok(global::inverse_lerp(float(a, 0)?, float(a, 1)?, float(a, 2)?).to_variant())
    }),
    ("remap", 5, |a| {
        let (value, istart, istop) = (float(a, 0)?, float(a, 1)?, float(a, 2)?);
        Ok(global::remap(value, istart, istop, float(a, 3)?, float(a, 4)?).to_variant())
    }),
    ("move_toward", 3, |a| {
        Ok(global::move_toward(float(a, 0)?, float(a, 1)?, float(a, 2)?).to_variant())
    }),
    ("rotate_toward", 3, |a| {
        Ok(global::rotate_toward(float(a, 0)?, float(a, 1)?, float(a, 2)?).to_variant())
    }),
    ("smoothstep", 3, |a| {
        Ok(global::smoothstep(float(a, 0)?, float(a, 1)?, float(a, 2)?).to_variant())
    }),
    ("ease", 2, |a| {
        Ok(global::ease(float(a, 0)?, float(a, 1)?).to_variant())
    }),
    ("lerp_angle", 3, |a| {
        Ok(global::lerp_angle(float(a, 0)?, float(a, 1)?, float(a, 2)?).to_variant())
    }),
    ("angle_difference", 2, |a| {
        Ok(global::angle_difference(float(a, 0)?, float(a, 1)?).to_variant())
    }),
    ("deg_to_rad", 1, |a| {
        Ok(global::deg_to_rad(float(a, 0)?).to_variant())
    }),
    ("rad_to_deg", 1, |a| {
        Ok(global::rad_to_deg(float(a, 0)?).to_variant())
    }),
    ("wrap", 3, |a| Ok(global::wrap(&a[0], &a[1], &a[2]))),
    ("wrapi", 3, |a| {
        Ok(global::wrapi(int(a, 0)?, int(a, 1)?, int(a, 2)?).to_variant())
    }),
    ("wrapf", 3, |a| {
        Ok(global::wrapf(float(a, 0)?, float(a, 1)?, float(a, 2)?).to_variant())
    }),
    ("snapped", 2, |a| Ok(global::snapped(&a[0], &a[1]))),
    ("pingpong", 2, |a| {
        Ok(global::pingpong(float(a, 0)?, float(a, 1)?).to_variant())
    }),
    ("clamp", 3, |a| Ok(global::clamp(&a[0], &a[1], &a[2]))),
    ("is_equal_approx", 2, |a| {
        Ok(global::is_equal_approx(float(a, 0)?, float(a, 1)?).to_variant())
    }),
    ("is_zero_approx", 1, |a| {
        Ok(global::is_zero_approx(float(a, 0)?).to_variant())
    }),
    ("fposmod", 2, |a| {
        Ok(global::fposmod(float(a, 0)?, float(a, 1)?).to_variant())
    }),
    ("fmod", 2, |a| {
        Ok(global::fmod(float(a, 0)?, float(a, 1)?).to_variant())
    }),
    ("linear_to_db", 1, |a| {
        Ok(global::linear_to_db(float(a, 0)?).to_variant())
    }),
    ("db_to_linear", 1, |a| {
        Ok(global::db_to_linear(float(a, 0)?).to_variant())
    }),
    ("randf", 0, |_| Ok(global::randf().to_variant())),
    ("randi", 0, |_| Ok(global::randi().to_variant())),
    ("randi_range", 2, |a| {
        Ok(global::randi_range(int(a, 0)?, int(a, 1)?).to_variant())
    }),
    ("randf_range", 2, |a| {
        Ok(global::randf_range(float(a, 0)?, float(a, 1)?).to_variant())
    }),
    ("randfn", 2, |a| {
        Ok(global::randfn(float(a, 0)?, float(a, 1)?).to_variant())
    }),
    ("seed", 1, |a| {
        global::seed(int(a, 0)?);
        Ok(Variant::nil())
    }),
    ("randomize", 0, |_| {
        global::randomize();
        Ok(Variant::nil())
    }),
    ("rand_from_seed", 1, |a| {
        Ok(global::rand_from_seed(int(a, 0)?).to_variant())
    }),
    ("push_error", ANY, |a| {
        global::push_error(a);
        Ok(Variant::nil())
    }),
    ("push_warning", ANY, |a| {
        global::push_warning(a);
        Ok(Variant::nil())
    }),
    ("is_same", 2, |a| {
        Ok(global::is_same(&a[0], &a[1]).to_variant())
    }),
];

// The argument at `index` as a float, converted as the engine converts one.
fn float(args: &[Variant], index: usize) -> Result<f64, String> {
    args[index]
        .try_to_relaxed::<f64>()
        .map_err(|_| conversion_failure(&args[index], index, VariantType::FLOAT))
}

// The argument at `index` as an integer, converted as the engine converts one.
fn int(args: &[Variant], index: usize) -> Result<i64, String> {
    args[index]
        .try_to_relaxed::<i64>()
        .map_err(|_| conversion_failure(&args[index], index, VariantType::INT))
}

fn conversion_failure(arg: &Variant, index: usize, to: VariantType) -> String {
    let name = |kind: VariantType| global::type_string(i64::from(kind.ord)).to_string();
    format!(
        "Cannot convert argument {} from {} to {}.",
        index + 1,
        name(arg.get_type()),
        name(to)
    )
}

/// The names of the utility functions, for `Godot` to define.
pub fn utilities(mrb: &Mrb, _godot: Value) -> Value {
    let names: Vec<Value> = UTILITIES
        .iter()
        .filter_map(|(name, _, _)| mrb.intern(name.as_bytes()).ok())
        .map(|id| Symbol::from(id).as_value())
        .collect();
    mrb.ary_new_from_values(&names).as_value()
}

/// Godot.__utility__(name, args): the utility function `name` called with
/// `args`.
pub fn utility(mrb: &Mrb, _godot: Value, name: Symbol, args: Array) -> Result<Value, Error> {
    let name = name.name(mrb).unwrap_or_default();
    let Some((_, arity, utility)) = UTILITIES.iter().find(|(row, _, _)| *row == name) else {
        return Ok(Value::nil());
    };
    if *arity != ANY && args.len() != *arity {
        let message = format!(
            "wrong number of arguments (given {}, expected {arity})",
            args.len()
        );
        return Err(match mrb.exc_get(c"ArgumentError") {
            Ok(class) => Error::new(mrb, class, &message),
            Err(error) => error,
        });
    }
    let args = args
        .entries(mrb)
        .map(|arg| value::to_engine(mrb, arg, 1))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|reason| call_error(mrb, &reason))?;
    let answer = utility(&args).map_err(|reason| {
        call_error(
            mrb,
            &format!("Invalid type in utility function '{name}'. {reason}"),
        )
    })?;
    ToRuby::try_new(&answer)
        .map(|answer| answer.into_value(mrb))
        .map_err(|reason| call_error(mrb, &reason))
}
