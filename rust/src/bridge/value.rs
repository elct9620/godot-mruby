//! The values that cross between the engine and Ruby, either way: data is
//! copied and objects are shared. A container crosses whole or not at all,
//! and one nested deeper than the engine's own recursive operations go
//! crosses as nothing.

use beni::{
    Array, Error, FromValue, Hash, IntoValue, Mrb, Proc, RString, ReprValue, Symbol, TryConvert,
    Value,
};
use godot::builtin::{
    AnyArray, AnyDictionary, Color, GString, PackedArray, StringName, VarArray, VarDictionary,
    Variant, VariantType, Vector2, Vector3, Vector4,
};
use godot::classes::Object;
use godot::meta::{PackedElement, ToGodot};
use godot::obj::Gd;

use crate::realm;

use super::object::{self, EngineObject};
use super::ruby_object::{self, RubyObject};
use super::value_type::{self, EngineValue};

/// How deep containers nest before a value stops crossing: the depth
/// Godot's own recursive Array and Dictionary operations stop at.
const DEPTH: usize = 100;

/// An engine value as Ruby is given it.
pub struct ToRuby<'a>(&'a Variant);

impl<'a> ToRuby<'a> {
    /// `variant` for Ruby, or why it cannot reach Ruby.
    pub fn checked(variant: &'a Variant) -> Result<Self, String> {
        match unreachable_from_engine(variant) {
            Some(reason) => Err(reason),
            None => Ok(Self(variant)),
        }
    }
}

impl IntoValue for ToRuby<'_> {
    fn into_value(self, mrb: &Mrb) -> Value {
        to_ruby(mrb, self.0)
    }
}

// Why an engine value cannot reach Ruby, if it cannot: a container nested
// too deep, or holding itself.
fn unreachable_from_engine(variant: &Variant) -> Option<String> {
    fn depth(variant: &Variant, level: usize) -> bool {
        if level > DEPTH {
            return false;
        }
        match variant.get_type() {
            VariantType::ARRAY => variant
                .to::<AnyArray>()
                .iter_shared()
                .all(|element| depth(&element, level + 1)),
            VariantType::DICTIONARY => variant
                .to::<AnyDictionary>()
                .iter_shared()
                .all(|(key, value)| depth(&key, level + 1) && depth(&value, level + 1)),
            _ => true,
        }
    }
    (!depth(variant, 1)).then(|| {
        let kind = godot::global::type_string(i64::from(variant.get_type().ord));
        format!("an {kind} nested more than {DEPTH} deep cannot reach Ruby")
    })
}

fn to_ruby(mrb: &Mrb, variant: &Variant) -> Value {
    match variant.get_type() {
        VariantType::BOOL => variant.to::<bool>().into_value(mrb),
        VariantType::INT => variant.to::<i64>().into_value(mrb),
        VariantType::FLOAT => variant.to::<f64>().into_value(mrb),
        VariantType::STRING => mrb
            .str_new(variant.to::<GString>().to_string().as_bytes())
            .as_value(),
        VariantType::STRING_NAME => symbol(mrb, &variant.to::<StringName>().to_string()),
        VariantType::ARRAY => array(mrb, variant.to::<AnyArray>().iter_shared()),
        VariantType::DICTIONARY => {
            let hash = mrb.hash_new();
            for (key, value) in variant.to::<AnyDictionary>().iter_shared() {
                hash.set(mrb, to_ruby(mrb, &key), to_ruby(mrb, &value)).ok();
            }
            hash.as_value()
        }
        VariantType::OBJECT => variant
            .try_to::<Gd<Object>>()
            .ok()
            .filter(Gd::is_instance_valid)
            .map_or_else(Value::nil, |object| match object.try_cast::<RubyObject>() {
                Ok(held) => realm::held(mrb, held.bind().key()).unwrap_or_else(Value::nil),
                Err(object) => object::ruby_object(mrb, object),
            }),
        kind if value_type::is_value_type(kind) => value_type::ruby_value(mrb, variant),
        VariantType::PACKED_BYTE_ARRAY => packed::<u8>(mrb, variant),
        VariantType::PACKED_INT32_ARRAY => packed::<i32>(mrb, variant),
        VariantType::PACKED_INT64_ARRAY => packed::<i64>(mrb, variant),
        VariantType::PACKED_FLOAT32_ARRAY => packed::<f32>(mrb, variant),
        VariantType::PACKED_FLOAT64_ARRAY => packed::<f64>(mrb, variant),
        VariantType::PACKED_STRING_ARRAY => packed::<GString>(mrb, variant),
        VariantType::PACKED_VECTOR2_ARRAY => packed::<Vector2>(mrb, variant),
        VariantType::PACKED_VECTOR3_ARRAY => packed::<Vector3>(mrb, variant),
        VariantType::PACKED_COLOR_ARRAY => packed::<Color>(mrb, variant),
        VariantType::PACKED_VECTOR4_ARRAY => packed::<Vector4>(mrb, variant),
        _ => Value::nil(),
    }
}

fn symbol(mrb: &Mrb, name: &str) -> Value {
    mrb.intern(name.as_bytes())
        .map_or_else(|_| Value::nil(), |id| Symbol::from(id).as_value())
}

fn array(mrb: &Mrb, elements: impl Iterator<Item = Variant>) -> Value {
    let values: Vec<Value> = elements.map(|element| to_ruby(mrb, &element)).collect();
    mrb.ary_new_from_values(&values).as_value()
}

fn packed<T: PackedElement>(mrb: &Mrb, variant: &Variant) -> Value {
    let packed = variant.to::<PackedArray<T>>();
    array(mrb, packed.as_slice().iter().map(ToGodot::to_variant))
}

/// A Ruby value as the engine takes it. Its Variant follows the Ruby value's
/// own class, so a Float stays a float where a converting read would take
/// an integer from it.
pub struct ToEngine(pub Variant);

impl TryConvert for ToEngine {
    fn try_convert(value: Value, mrb: &Mrb) -> Result<Self, Error> {
        to_engine(mrb, value, 1)
            .map(Self)
            .map_err(|reason| match mrb.exc_get(c"TypeError") {
                Ok(class) => Error::new(mrb, class, &reason),
                Err(error) => error,
            })
    }
}

/// `value` as the engine takes it, or why it cannot reach the engine.
pub fn to_engine(mrb: &Mrb, value: Value, level: usize) -> Result<Variant, String> {
    if value.is_nil() {
        return Ok(Variant::nil());
    }
    if value.is_true() || value.is_false() {
        return Ok(value.is_true().to_variant());
    }
    if let Some(integer) = i64::from_value(value) {
        return Ok(integer.to_variant());
    }
    if let Some(float) = f64::from_value(value) {
        return Ok(float.to_variant());
    }
    if let Some(string) = RString::from_value(value) {
        let text = string.to_string(mrb).map_err(|error| error.message(mrb))?;
        return Ok(GString::from(&text).to_variant());
    }
    if let Some(symbol) = Symbol::from_value(value) {
        let name = symbol.name(mrb).unwrap_or_default();
        return Ok(StringName::from(&name).to_variant());
    }
    if let Ok(held) = <&EngineObject>::try_convert(value, mrb) {
        return held.variant();
    }
    if let Ok(held) = <&EngineValue>::try_convert(value, mrb) {
        return Ok(held.variant());
    }
    let too_deep = || {
        let class = value.classname(mrb);
        format!("an {class} nested more than {DEPTH} deep cannot reach the engine")
    };
    if let Some(array) = Array::from_value(value) {
        if level > DEPTH {
            return Err(too_deep());
        }
        let mut copied = VarArray::new();
        for element in array.entries(mrb) {
            copied.push(&to_engine(mrb, element, level + 1)?);
        }
        return Ok(copied.to_variant());
    }
    if let Some(hash) = Hash::from_value(value) {
        if level > DEPTH {
            return Err(too_deep());
        }
        let mut copied = VarDictionary::new();
        let keys = hash.keys(mrb);
        for key in keys.entries(mrb) {
            let entry = hash.get(mrb, key).map_err(|error| error.message(mrb))?;
            copied.set(
                &to_engine(mrb, key, level + 1)?,
                &to_engine(mrb, entry, level + 1)?,
            );
        }
        return Ok(copied.to_variant());
    }
    let class = value.classname(mrb);
    let key = realm::hold_new(mrb, value).map_err(|error| error.message(mrb))?;
    if Proc::from_value(value).is_some() || class == "Method" {
        return Ok(ruby_object::callable(key, class).to_variant());
    }
    Ok(RubyObject::holding(key, &class).to_variant())
}
