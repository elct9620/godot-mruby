//! A `StringName` as a map key, compared and hashed by the interned name it
//! points to, as Godot itself compares two names, so a lookup asks the
//! engine nothing.

use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};

use godot::builtin::StringName;

/// A name a map is keyed by. It holds the name, so the interned name it
/// points to is never freed and taken by another while the key is kept, and
/// a map keyed by it is looked up by `read_identity` of a name it was not
/// handed.
#[derive(Clone)]
pub struct NameKey {
    name: StringName,
    identity: usize,
}

impl NameKey {
    pub fn new(name: StringName) -> Self {
        let identity = read_identity(&name);
        Self { name, identity }
    }
}

/// The interned name `name` points to. Godot's StringName holds one pointer
/// to its interned data, and two names are equal exactly when they hold the
/// same pointer.
pub fn read_identity(name: &StringName) -> usize {
    // SAFETY: the StringName's opaque storage is that one pointer, alive
    // while `name` is borrowed.
    unsafe { *name.string_sys().cast::<usize>() }
}

impl PartialEq for NameKey {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
}

impl Eq for NameKey {}

impl Hash for NameKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identity.hash(state);
    }
}

impl Borrow<usize> for NameKey {
    fn borrow(&self) -> &usize {
        &self.identity
    }
}

impl fmt::Display for NameKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}
