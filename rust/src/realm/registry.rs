//! The Ruby objects a realm holds for what lives outside it. Something outside
//! keeps a key, and the object stays reachable from a hash the collector
//! always marks, so mruby never frees what the engine still uses.

use std::cell::Cell;

use beni::{Error, Hash, Mrb, Value};

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

    /// The object `key` holds, or nil when it holds none.
    pub fn object(&self, mrb: &Mrb, key: Key) -> Result<Value, Error> {
        self.objects.get(mrb, key.to_value(mrb))
    }
}

impl Key {
    // beni has `IntoValue` for i32 but not i64, so the key goes through
    // `Value::from_int`.
    fn to_value(self, mrb: &Mrb) -> Value {
        Value::from_int(mrb, self.0)
    }
}
