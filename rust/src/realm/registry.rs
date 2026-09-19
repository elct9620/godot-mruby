//! The Ruby objects a realm holds for what lives outside it. Something outside
//! keeps a key, and the object stays reachable from a hash the collector
//! always marks, so mruby never frees what the engine still uses.

use std::cell::Cell;

use beni::{Error, Hash, IntoValue, Mrb, ReprValue, Value};

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
    objects: Hash,
    // The last key the registry named itself. It counts down from -1, since
    // a node's key is its instance id, which is positive.
    last: Cell<i64>,
}

impl Registry {
    pub fn new(mrb: &Mrb) -> Self {
        let objects = mrb.hash_new();
        mrb.gc_register_forever(objects.as_value());
        Self {
            objects,
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
        self.objects.set(mrb, key.to_value(mrb), object)
    }

    /// Whether `key` holds an object.
    pub fn holds(&self, mrb: &Mrb, key: Key) -> Result<bool, Error> {
        self.objects.contains_key(mrb, key.to_value(mrb))
    }

    /// The object `key` holds; a key released already holds none.
    pub fn object(&self, mrb: &Mrb, key: Key) -> Result<Value, Error> {
        if !self.holds(mrb, key)? {
            let class = mrb.exc_get(c"RuntimeError")?;
            return Err(Error::new(mrb, class, "the object was released"));
        }
        self.objects.get(mrb, key.to_value(mrb))
    }

    /// Lets go of the object `key` holds, for the collector to free.
    pub fn release(&self, mrb: &Mrb, key: Key) {
        self.objects.delete(mrb, key.to_value(mrb)).ok();
    }

    #[cfg(test)]
    pub fn len(&self, mrb: &Mrb) -> usize {
        self.objects.len(mrb)
    }
}

impl Key {
    fn to_value(self, mrb: &Mrb) -> Value {
        self.0.into_value(mrb)
    }
}
