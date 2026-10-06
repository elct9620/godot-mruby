//! An engine method found once by the hash the engine gives its signature,
//! so a call reaches the method's bind without the engine looking up its
//! name.

use std::cell::Cell;

use beni::{DataType, Mrb, RClass, TypedData};
use godot::builtin::{Array, GString, StringName, VarArray, VarDictionary, Variant, VariantType};
use godot::classes::class_db::ApiType;
use godot::classes::{ClassDb, Object};
use godot::obj::{EngineBitfield, Gd, Singleton};
use godot::register::info::{MethodFlags, PropertyUsageFlags};
use godot::sys;
use smallvec::SmallVec;

use crate::realm;

/// The bind of an engine method a class the engine registered itself
/// declares, which lives until the engine shuts down; an extension's binds
/// are freed when it unloads, so none is held.
pub struct BoundMethod {
    bind: sys::GDExtensionMethodBindPtr,
    name: String,
    arity: Arity,
}

/// How many arguments an engine method takes: those it requires, those
/// with a default after them, and whether any number may follow.
#[derive(Clone, Copy)]
pub struct Arity {
    pub required: usize,
    pub optional: usize,
    pub is_vararg: bool,
}

// SAFETY: the bind is the engine's, unchanged while the engine runs, and a
// realm is entered by one thread at a time.
unsafe impl Send for BoundMethod {}

static BOUND_METHOD: DataType<BoundMethod> = DataType::new(c"Godot::BoundMethod");

// SAFETY: `class` marks the class it makes, the only one a bound method is
// wrapped as.
unsafe impl TypedData for BoundMethod {
    fn class(mrb: &Mrb) -> RClass {
        let kept = &realm::extension_data::<BoundClass>(mrb).0;
        super::find_class_once(mrb, kept, || {
            let class = mrb
                .class_new(mrb.object_class())
                .expect("mruby makes a class");
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

// The class of bound methods, nameless so it takes no name under Godot,
// made once for a realm and kept for it.
#[derive(Default)]
struct BoundClass(Cell<Option<RClass>>);

impl BoundMethod {
    /// The method `method` of the engine class `class`, bound where the
    /// nearest of the class and its ancestors declares it, or none when an
    /// extension registered that one.
    pub fn find(class: &str, method: &str) -> Option<Self> {
        let class_db = ClassDb::singleton();
        let declarer = find_declarer(&class_db, class, method)?;
        let api = class_db.class_get_api_type(&declarer);
        if api != ApiType::CORE && api != ApiType::EDITOR {
            return None;
        }
        let info = find_info(&class_db, &declarer, method)?;
        let hash = hash_method(&info)?;
        let arity = Arity::read(&info)?;
        let name = StringName::from(method);
        // SAFETY: the interface is initialized while the extension runs, and
        // the names live for the call.
        let bind = unsafe {
            sys::interface_fn!(classdb_get_method_bind)(
                declarer.string_sys(),
                name.string_sys(),
                i64::from(hash),
            )
        };
        (!bind.is_null()).then(|| Self {
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

    /// Calls the method on `object`, which must be of the class the method
    /// was found for or a class extending it.
    pub fn call(
        &self,
        object: &Gd<Object>,
        args: &[Variant],
    ) -> Result<Variant, sys::GDExtensionCallError> {
        let pointers: SmallVec<[_; 4]> = args.iter().map(Variant::var_sys).collect();
        // SAFETY: the bind is alive while the engine runs, `object` is a live
        // object of a class the bind's class is or extends, and the argument
        // pointers live for the call.
        super::run_engine_call(|answer, error| unsafe {
            sys::interface_fn!(object_method_bind_call)(
                self.bind,
                object.obj_sys(),
                pointers.as_ptr(),
                pointers.len() as i64,
                answer,
                error,
            );
        })
    }
}

impl Arity {
    /// The arity of the engine method `method` of the engine class `class`,
    /// or of the nearest of its ancestors declaring it.
    pub fn find(class: &str, method: &str) -> Option<Self> {
        let class_db = ClassDb::singleton();
        let declarer = find_declarer(&class_db, class, method)?;
        Self::read(&find_info(&class_db, &declarer, method)?)
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

    // Words a method's signature gives its hash, as hash_method collects
    // them, beside the hash Godot 4.6's extension API lists for the method.
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
