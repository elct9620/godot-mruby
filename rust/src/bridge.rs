//! What Ruby sees of the engine: the `Godot` module, a gem every game realm
//! opens with, and the values that cross between the engine and Ruby.

use beni::{Error, FromValue, Gem, IntoValue, Mrb, Object, Symbol, Value, method};
use godot::builtin::{Variant, VariantType};
use godot::classes::ClassDb;
use godot::meta::ToGodot;
use godot::obj::Singleton;

use crate::{compiler, log};

const FILE: &std::ffi::CStr = c"godot_mruby/bridge/godot.rb";

pub struct Godot;

impl Gem for Godot {
    fn init(mrb: &Mrb) -> Result<(), Error> {
        mrb.define_module(c"Godot")?.define_singleton_method(
            mrb,
            c"__engine_superclass__",
            method!(engine_superclass, 1),
        )?;
        compiler::run(
            mrb,
            FILE,
            include_str!("bridge/godot.rb"),
            log::compiler_warnings(FILE),
        )
    }
}

// Godot.__engine_superclass__(name): the name of the engine class `name`
// inherits from, empty for the root, or nil when the engine has no such class.
fn engine_superclass(mrb: &Mrb, _godot: Value, name: Symbol) -> Value {
    let class_db = ClassDb::singleton();
    let Some(name) = name.name(mrb).filter(|name| class_db.class_exists(name)) else {
        return Value::nil();
    };
    let parent = class_db.get_parent_class(&name).to_string();
    mrb.str_new(parent.as_bytes()).as_value()
}

/// An engine value as Ruby is given it. Only `nil`, booleans, integers and
/// floats cross yet; anything else arrives as `nil`.
pub struct Argument<'a>(pub &'a Variant);

impl IntoValue for Argument<'_> {
    fn into_value(self, mrb: &Mrb) -> Value {
        match self.0.get_type() {
            VariantType::BOOL => self.0.to::<bool>().into_value(mrb),
            // beni has `IntoValue` for i32 but not i64, so an integer goes
            // through `Value::from_int`.
            VariantType::INT => Value::from_int(mrb, self.0.to::<i64>()),
            VariantType::FLOAT => self.0.to::<f64>().into_value(mrb),
            _ => Value::nil(),
        }
    }
}

/// What Ruby answered, as the engine takes it. Only `nil`, booleans, integers
/// and floats cross yet; anything else reaches the engine as null, since
/// what a callback answers is mostly the value of its last line.
pub struct Answer(pub Variant);

impl FromValue for Answer {
    fn from_value(value: Value) -> Option<Self> {
        let variant = if value.is_true() {
            true.to_variant()
        } else if value.is_false() {
            false.to_variant()
        } else if let Some(integer) = i64::from_value(value) {
            integer.to_variant()
        } else if let Some(float) = f64::from_value(value) {
            float.to_variant()
        } else {
            Variant::nil()
        };
        Some(Self(variant))
    }
}
