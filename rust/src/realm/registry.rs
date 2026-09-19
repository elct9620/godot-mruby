//! The Ruby objects a realm holds for what lives outside it. Something outside
//! keeps a key, and the object stays reachable from a hash the collector
//! always marks, so mruby never frees what the engine still uses.

use std::cell::Cell;

use beni::{Error, Hash, IntoValue, Mrb, ReprValue, Value};

/// A Ruby object a realm holds for something outside it, which keeps the key
/// rather than the object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key(i64);

pub(super) struct Registry {
    objects: Hash,
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

    /// Holds `object` under a key no other object has had in this realm.
    pub fn hold(&self, mrb: &Mrb, object: Value) -> Result<Key, Error> {
        let key = Key(self.last.get() + 1);
        self.objects.set(mrb, key.to_value(mrb), object)?;
        self.last.set(key.0);
        Ok(key)
    }

    /// The object `key` holds; a key released already holds none.
    pub fn object(&self, mrb: &Mrb, key: Key) -> Result<Value, Error> {
        let held = key.to_value(mrb);
        if !self.objects.contains_key(mrb, held)? {
            let class = mrb.exc_get(c"RuntimeError")?;
            return Err(Error::new(mrb, class, "the object was released"));
        }
        self.objects.get(mrb, held)
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
