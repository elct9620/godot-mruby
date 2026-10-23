//! An engine method found once by the hash the engine gives its signature,
//! so a call reaches the method's bind without the engine looking up its
//! name, or, for an extension's class, by the hash kept for each call.

use std::ffi::c_void;

use beni::{Class as _, DataType, Mrb, RClass, TypedData};
use godot::builtin::{Array, GString, StringName, VarArray, VarDictionary, Variant, VariantType};
use godot::classes::class_db::ApiType;
use godot::classes::{ClassDb, Object};
use godot::obj::{EngineBitfield, Gd, Singleton};
use godot::register::info::{MethodFlags, PropertyUsageFlags};
use godot::sys;
use smallvec::SmallVec;

/// An engine method found by the hash the engine gives its signature, whose
/// bind a call reaches without the engine asking the receiver's script.
pub struct BoundMethod {
    bind: Bind,
    name: String,
    arity: Arity,
}

// Where a call finds the method's bind and the tag of the class declaring
// it, which its receiver must be of or extend, since the bind takes the
// receiver as that class.
enum Bind {
    // A class the engine registered itself, whose bind and tag live until
    // the engine shuts down.
    Engine {
        bind: sys::GDExtensionMethodBindPtr,
        declarer_tag: *mut c_void,
    },
    // An extension's class, whose bind is freed when the extension unloads,
    // so each call asks the engine for it again.
    Extension {
        declarer: String,
        hash: u32,
    },
}

/// How many arguments an engine method takes: those it requires, those
/// with a default after them, and whether any number may follow.
#[derive(Clone, Copy)]
pub struct Arity {
    pub required: usize,
    pub optional: usize,
    pub is_vararg: bool,
}

// SAFETY: an engine class's bind and tag are the engine's, unchanged while
// the engine runs, an extension's are asked for at each call, and a realm is
// entered by one thread at a time.
unsafe impl Send for BoundMethod {}

static BOUND_METHOD: DataType<BoundMethod> = DataType::new(c"Godot::BoundMethod");

// SAFETY: `class` marks the class it makes, the only one a bound method is
// wrapped as.
unsafe impl TypedData for BoundMethod {
    fn class(mrb: &Mrb) -> RClass {
        super::find_class_once(mrb, &super::data(mrb).bound_method_class, || {
            let class = RClass::new(mrb, mrb.class_object()).expect("mruby makes a class");
            class
                .set_instance_data_tt(mrb)
                .expect("mruby marks a class's data");
            class
        })
    }

    fn data_type() -> &'static DataType<Self> {
        &BOUND_METHOD
    }
}

impl BoundMethod {
    /// The method `method` of the engine class `class`, bound where the
    /// nearest of the class and its ancestors declares it.
    pub fn find(class: &str, method: &str) -> Option<Self> {
        let class_db = ClassDb::singleton();
        let declarer = find_declarer(&class_db, class, method)?;
        let info = find_info(&class_db, &declarer, method)?;
        let hash = hash_method(&info)?;
        let arity = Arity::read(&info)?;
        let (bind, declarer_tag) = find_bind(&declarer, &StringName::from(method), hash)?;
        let api = class_db.class_get_api_type(&declarer);
        let bind = if api == ApiType::CORE || api == ApiType::EDITOR {
            Bind::Engine { bind, declarer_tag }
        } else {
            Bind::Extension {
                declarer: declarer.to_string(),
                hash,
            }
        };
        Some(Self {
            bind,
            name: method.to_owned(),
            arity,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn arity(&self) -> Arity {
        self.arity
    }

    /// Calls the method on `object`, which the caller found live with
    /// nothing run since, Ruby included. A receiver of a class neither
    /// declaring the method nor extending one that does, and a count of
    /// arguments the method does not take, are refused here: the bind trusts
    /// both, and a game exported without the engine's debug checks checks
    /// neither.
    pub fn call(
        &self,
        object: &Gd<Object>,
        args: &[Variant],
    ) -> Result<Variant, sys::GDExtensionCallError> {
        let (bind, declarer_tag) = self.bind.find(&self.name).ok_or(INVALID_METHOD)?;
        check_receiver(object, declarer_tag)?;
        self.arity.check(args.len())?;
        let pointers: SmallVec<[_; 4]> = args.iter().map(Variant::var_sys).collect();
        // SAFETY: the bind is alive while the engine runs or, for an
        // extension's class, was found just now with nothing run since,
        // `object` is a live object of the bind's class or one extending it,
        // as checked, and the argument pointers live for the call.
        super::run_engine_call(|answer, error| unsafe {
            sys::interface_fn!(object_method_bind_call)(
                bind,
                object.obj_sys(),
                pointers.as_ptr(),
                pointers.len() as i64,
                answer,
                error,
            );
        })
    }
}

impl Bind {
    // The method `method`'s bind and its declarer's tag, or none once the
    // extension declaring it has unloaded.
    fn find(&self, method: &str) -> Option<(sys::GDExtensionMethodBindPtr, *mut c_void)> {
        match self {
            Self::Engine { bind, declarer_tag } => Some((*bind, *declarer_tag)),
            Self::Extension { declarer, hash } => find_bind(
                &StringName::from(declarer.as_str()),
                &StringName::from(method),
                *hash,
            ),
        }
    }
}

// The error the engine answers for a method its receiver's class lacks.
const INVALID_METHOD: sys::GDExtensionCallError = sys::GDExtensionCallError {
    error: sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD,
    argument: 0,
    expected: 0,
};

// Whether `object` is of the class `declarer_tag` tags or one extending it,
// or the error the engine answers for a method its class lacks.
fn check_receiver(
    object: &Gd<Object>,
    declarer_tag: *mut c_void,
) -> Result<(), sys::GDExtensionCallError> {
    // SAFETY: the interface is initialized while the extension runs, and
    // `object` is live.
    let cast = unsafe { sys::interface_fn!(object_cast_to)(object.obj_sys(), declarer_tag) };
    if cast.is_null() {
        return Err(INVALID_METHOD);
    }
    Ok(())
}

// The bind the engine registered for `method` of `declarer` under `hash`,
// with `declarer`'s tag, or none when it has neither.
fn find_bind(
    declarer: &StringName,
    method: &StringName,
    hash: u32,
) -> Option<(sys::GDExtensionMethodBindPtr, *mut c_void)> {
    // SAFETY: the interface is initialized while the extension runs, and the
    // names live for the call.
    let (bind, tag) = unsafe {
        (
            sys::interface_fn!(classdb_get_method_bind)(
                declarer.string_sys(),
                method.string_sys(),
                i64::from(hash),
            ),
            sys::interface_fn!(classdb_get_class_tag)(declarer.string_sys()),
        )
    };
    (!bind.is_null() && !tag.is_null()).then_some((bind, tag))
}

impl Arity {
    /// The arity of the engine method `method` of the engine class `class`,
    /// or of the nearest of its ancestors declaring it.
    pub fn find(class: &str, method: &str) -> Option<Self> {
        let class_db = ClassDb::singleton();
        let declarer = find_declarer(&class_db, class, method)?;
        Self::read(&find_info(&class_db, &declarer, method)?)
    }

    /// Whether the method takes `given` arguments, or the error the engine's
    /// debug checks answer when it does not.
    pub fn check(&self, given: usize) -> Result<(), sys::GDExtensionCallError> {
        let takes = self.required + self.optional;
        let error = if given < self.required {
            sys::GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS
        } else if given > takes && !self.is_vararg {
            sys::GDEXTENSION_CALL_ERROR_TOO_MANY_ARGUMENTS
        } else {
            return Ok(());
        };
        Err(sys::GDExtensionCallError {
            error,
            argument: 0,
            expected: takes as i32,
        })
    }

    // The arity a method's `info`, as ClassDB lists it, gives.
    fn read(info: &VarDictionary) -> Option<Self> {
        let taken = info
            .get_or_nil("args")
            .try_to::<Array<VarDictionary>>()
            .ok()?
            .len();
        let optional = info
            .get_or_nil("default_args")
            .try_to::<VarArray>()
            .ok()?
            .len();
        let flags = info.get_or_nil("flags").try_to::<i64>().ok()? as u64;
        Some(Self {
            required: taken.saturating_sub(optional),
            optional,
            is_vararg: flags & MethodFlags::VARARG.ord() != 0,
        })
    }
}

// The info ClassDB lists for `method` among the own methods of `declarer`.
fn find_info(class_db: &Gd<ClassDb>, declarer: &StringName, method: &str) -> Option<VarDictionary> {
    class_db
        .class_get_method_list_ex(declarer)
        .no_inheritance(true)
        .done()
        .iter_shared()
        .find(|info| info.get_or_nil("name").to_string() == method)
}

#[cfg(feature = "editor")]
/// The engine class that binds `method` among `class` and its ancestors,
/// nearest first, as GDScript names the native class a method overrides.
pub fn find_method_declarer(class: &str, method: &str) -> Option<String> {
    find_declarer(&ClassDb::singleton(), class, method).map(|declarer| declarer.to_string())
}

#[cfg(feature = "editor")]
/// Whether `member`, a property or a method of the engine class `class` or
/// one of its ancestors, answers a StringName.
pub fn is_string_name_member(class: &str, member: &str) -> bool {
    let class_db = ClassDb::singleton();
    let is_string_name = |info: &VarDictionary| {
        info.get_or_nil("type").try_to::<i64>().ok()
            == Some(i64::from(VariantType::STRING_NAME.ord))
    };
    let property = class_db
        .class_get_property_list(class)
        .iter_shared()
        .find(|info| info.get_or_nil("name").to_string() == member);
    if let Some(property) = property {
        return is_string_name(&property);
    }
    find_declarer(&class_db, class, member)
        .and_then(|declarer| find_info(&class_db, &declarer, member))
        .and_then(|info| info.get_or_nil("return").try_to::<VarDictionary>().ok())
        .is_some_and(|returned| is_string_name(&returned))
}

/// Whether the engine class `class` or one of its ancestors declares a
/// static method `method`, which the class itself answers.
pub fn is_static_method(class: &str, method: &str) -> bool {
    let class_db = ClassDb::singleton();
    find_declarer(&class_db, class, method)
        .and_then(|declarer| find_info(&class_db, &declarer, method))
        .and_then(|info| info.get_or_nil("flags").try_to::<i64>().ok())
        .is_some_and(|flags| flags as u64 & MethodFlags::STATIC.ord() != 0)
}

// The nearest of the engine class `class` and its ancestors whose own
// methods include `method`.
fn find_declarer(class_db: &Gd<ClassDb>, class: &str, method: &str) -> Option<StringName> {
    let mut current = StringName::from(class);
    while !current.is_empty() {
        let declares = class_db
            .class_has_method_ex(&current, method)
            .no_inheritance(true)
            .done();
        if declares {
            return Some(current);
        }
        current = class_db.get_parent_class(&current);
    }
    None
}

// The hash the engine gives a method's signature, MethodInfo's
// get_compatibility_hash, from the method's `info` as ClassDB lists it.
fn hash_method(info: &VarDictionary) -> Option<u32> {
    let returned = info.get_or_nil("return").try_to::<VarDictionary>().ok()?;
    let args = info
        .get_or_nil("args")
        .try_to::<Array<VarDictionary>>()
        .ok()?;
    let defaults = info.get_or_nil("default_args").try_to::<VarArray>().ok()?;
    let flags = info.get_or_nil("flags").try_to::<i64>().ok()? as u64;
    let return_type = returned.get_or_nil("type").try_to::<i64>().ok()?;
    let usage = returned.get_or_nil("usage").try_to::<i64>().ok()? as u64;
    let has_return = return_type != i64::from(VariantType::NIL.ord)
        || usage & PropertyUsageFlags::NIL_IS_VARIANT.ord() != 0;
    let mut words = vec![u32::from(has_return), args.len() as u32];
    if has_return {
        push_type(&mut words, &returned)?;
    }
    for arg in args.iter_shared() {
        push_type(&mut words, &arg)?;
    }
    words.push(defaults.len() as u32);
    words.extend(defaults.iter_shared().map(|default| default.hash_u32()));
    words.push(u32::from(flags & MethodFlags::CONST.ord() != 0));
    words.push(u32::from(flags & MethodFlags::VARARG.ord() != 0));
    Some(hash_words(words))
}

// The words a value's type adds to a signature's hash: its variant type,
// and the hash of its class's name when it names one.
fn push_type(words: &mut Vec<u32>, info: &VarDictionary) -> Option<()> {
    words.push(info.get_or_nil("type").try_to::<i64>().ok()? as u32);
    let class_name = GString::from(&info.get_or_nil("class_name").try_to::<StringName>().ok()?);
    if !class_name.is_empty() {
        words.push(class_name.hash_u32());
    }
    Some(())
}

// MurmurHash3's 32-bit rounds over `words`, seeded and finished as the
// engine's hash_murmur3_one_32 and hash_fmix32 are.
fn hash_words(words: impl IntoIterator<Item = u32>) -> u32 {
    const SEED: u32 = 0x7F0_7C65;
    let hash = words.into_iter().fold(SEED, |hash, word| {
        let word = word
            .wrapping_mul(0xCC9E_2D51)
            .rotate_left(15)
            .wrapping_mul(0x1B87_3593);
        (hash ^ word)
            .rotate_left(13)
            .wrapping_mul(5)
            .wrapping_add(0xE654_6B64)
    });
    let hash = (hash ^ (hash >> 16)).wrapping_mul(0x85EB_CA6B);
    let hash = (hash ^ (hash >> 13)).wrapping_mul(0xC2B2_AE35);
    hash ^ (hash >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    // @behavior RG-037
    #[test]
    fn check_refuses_a_count_the_method_does_not_take() {
        let with_defaults = Arity {
            required: 1,
            optional: 2,
            is_vararg: false,
        };
        let any_number = Arity {
            required: 1,
            optional: 0,
            is_vararg: true,
        };

        let refusals = [
            (with_defaults, 0),
            (with_defaults, 1),
            (with_defaults, 3),
            (with_defaults, 4),
            (any_number, 0),
            (any_number, 9),
        ]
        .map(|(arity, given)| arity.check(given).err().map(|error| error.error));

        assert_eq!(
            refusals,
            [
                Some(sys::GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS),
                None,
                None,
                Some(sys::GDEXTENSION_CALL_ERROR_TOO_MANY_ARGUMENTS),
                Some(sys::GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS),
                None,
            ]
        );
    }

    // Words a method's signature gives its hash, as hash_method collects
    // them, beside the hash Godot 4.6's extension API lists for the method.
    // @behavior RG-048
    #[test]
    fn hash_words_gives_the_engines_hash_of_a_signature() {
        let getter = [1, 0, VariantType::INT.ord as u32, 0, 1, 0];
        let setter = [0, 1, VariantType::INT.ord as u32, 0, 0, 0];
        let vector_getter = [1, 0, VariantType::VECTOR2.ord as u32, 0, 1, 0];
        let vector_setter = [0, 1, VariantType::VECTOR2.ord as u32, 0, 0, 0];

        let hashes = [getter, setter, vector_getter, vector_setter].map(hash_words);

        // Node.get_process_priority, Node.set_process_priority,
        // Node2D.get_position and Node2D.set_position.
        assert_eq!(hashes, [3905245786, 1286410249, 3341600327, 743155724]);
    }
}
