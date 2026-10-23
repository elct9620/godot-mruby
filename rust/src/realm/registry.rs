//! The Ruby objects a realm holds for what lives outside it. Something outside
//! keeps a key, and the object stays reachable from a hash the collector
//! always marks, so mruby never frees what the engine still uses. An object
//! standing for a reference-counted engine object only Ruby holds is held
//! loosely instead: unmarked, so the collector frees it once Ruby no longer
//! reaches it, and the engine object goes with it.

use std::cell::{Cell, RefCell};
use std::sync::{Mutex, PoisonError};

use beni::{Error, IntoValue, Mrb, RHash, ReprValue, Value, value::Opaque};
use rustc_hash::{FxHashMap, FxHashSet};

// The keys whose loosely held object the collector has freed, noted as it
// frees it, so the registry never reaches the memory it was in.
static DEAD_KEYS: Mutex<Option<FxHashSet<i64>>> = Mutex::new(None);

/// Notes that the collector freed the object `key` holds loosely.
pub fn note_collected(key: Key) {
    DEAD_KEYS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get_or_insert_default()
        .insert(key.0);
}

/// Whether the collector freed the object `key` held loosely.
pub fn is_collected(key: Key) -> bool {
    DEAD_KEYS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .is_some_and(|keys| keys.contains(&key.0))
}

fn forget_collected(key: Key) {
    if let Some(keys) = DEAD_KEYS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
    {
        keys.remove(&key.0);
    }
}

/// What something outside a realm keeps for the Ruby object the realm holds
/// for it; the outside names it, as a node by its instance id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key(i64);

impl From<i64> for Key {
    fn from(key: i64) -> Self {
        Self(key)
    }
}

pub(super) struct Registry {
    objects: Opaque<RHash>,
    // The objects held loosely, by key: the collector does not mark them, so
    // one is read only while `DEAD_KEYS` does not name its key.
    loose: RefCell<FxHashMap<i64, Opaque<Value>>>,
    // The last key the registry named itself. It counts down from -1, since
    // a node's key is its instance id, which is positive: only a
    // reference-counted object's id is negative, and no node is one.
    last: Cell<i64>,
}

impl Registry {
    pub fn new(mrb: &Mrb) -> Self {
        let objects = mrb.hash_new();
        mrb.gc_register_mark_object(objects.as_value());
        Self {
            objects: objects.into(),
            loose: RefCell::default(),
            last: Cell::new(0),
        }
    }

    /// Holds `object` under a key the registry names, which no node's is.
    pub fn hold_new(&self, mrb: &Mrb, object: Value) -> Result<Key, Error> {
        let key = Key(self.last.get() - 1);
        self.hold(mrb, key, object)?;
        self.last.set(key.0);
        Ok(key)
    }

    /// Holds `object` under `key`.
    pub fn hold(&self, mrb: &Mrb, key: Key, object: Value) -> Result<(), Error> {
        mrb.get_inner(self.objects)
            .aset(mrb, key.to_value(mrb), object)
    }

    /// Holds `object` under `key` loosely, for an object standing for a
    /// reference-counted engine object only Ruby holds.
    pub fn hold_loose(&self, key: Key, object: Value) {
        forget_collected(key);
        self.loose.borrow_mut().insert(key.0, object.into());
    }

    /// Holds the object `key` holds loosely as firmly as any other, since
    /// the engine holds its engine object too; one the collector freed stays
    /// gone.
    pub fn tighten(&self, mrb: &Mrb, key: Key) -> Result<(), Error> {
        let Some(object) = self.loose.borrow_mut().remove(&key.0) else {
            return Ok(());
        };
        let object = mrb.get_inner(object);
        if is_collected(key) {
            return Ok(());
        }
        self.hold(mrb, key, object)
    }

    /// Holds the object `key` holds loosely from now on, since only Ruby
    /// holds its engine object.
    pub fn loosen(&self, mrb: &Mrb, key: Key) -> Result<(), Error> {
        let object: Value = mrb.get_inner(self.objects).delete(mrb, key.to_value(mrb))?;
        if !object.is_nil() {
            self.hold_loose(key, object);
        }
        Ok(())
    }

    /// Whether `key` holds an object.
    pub fn has_object(&self, mrb: &Mrb, key: Key) -> Result<bool, Error> {
        Ok(mrb
            .get_inner(self.objects)
            .contains_key(mrb, key.to_value(mrb))?
            || self.loose_object_by_key(mrb, key).is_some())
    }

    // The object `key` holds loosely while the collector has not freed it,
    // protected in the arena now that Rust has it: callers run inside an
    // arena scope, which takes the protection back as it ends.
    fn loose_object_by_key(&self, mrb: &Mrb, key: Key) -> Option<Value> {
        let object = mrb.get_inner(*self.loose.borrow().get(&key.0)?);
        (!is_collected(key)).then(|| mrb.arena_scope().keep(object))
    }

    /// The object `key` holds; a key released already holds none. A held
    /// object is never nil, so nil means the key holds none.
    pub fn object_by_key(&self, mrb: &Mrb, key: Key) -> Result<Value, Error> {
        let mut object: Value = mrb.get_inner(self.objects).aref(mrb, key.to_value(mrb))?;
        if object.is_nil() {
            object = self.loose_object_by_key(mrb, key).unwrap_or(object);
        }
        if object.is_nil() {
            let class = mrb.exc_get(c"RuntimeError")?;
            return Err(Error::new(mrb, class, "the object was released"));
        }
        Ok(object)
    }

    /// Every object held, loosely held ones the collector has not freed
    /// included.
    pub fn objects(&self, mrb: &Mrb) -> Vec<Value> {
        let keys: Vec<i64> = self.loose.borrow().keys().copied().collect();
        mrb.get_inner(self.objects)
            .values(mrb)
            .entries(mrb)
            .chain(
                keys.into_iter()
                    .filter_map(|key| self.loose_object_by_key(mrb, Key(key))),
            )
            .collect()
    }

    /// Lets go of the object `key` holds, for the collector to free.
    pub fn release(&self, mrb: &Mrb, key: Key) {
        mrb.get_inner(self.objects)
            .delete::<_, Value>(mrb, key.to_value(mrb))
            .ok();
        self.loose.borrow_mut().remove(&key.0);
        forget_collected(key);
    }

    #[cfg(test)]
    pub fn len(&self, mrb: &Mrb) -> usize {
        mrb.get_inner(self.objects).len()
    }
}

impl Key {
    fn to_value(self, mrb: &Mrb) -> Value {
        self.0.into_value(mrb)
    }
}
