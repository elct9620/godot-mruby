//! A `StringName` as a map key, compared and hashed by the interned name it
//! points to, as Godot itself compares two names, so a lookup asks the
//! engine nothing.

use std::fmt;
use std::hash::{Hash, Hasher};

use godot::builtin::StringName;

/// A name a map is keyed by. It holds the name, so the interned name it
/// points to is never freed and taken by another while the key is kept.
#[derive(Clone)]
pub struct NameKey(StringName);

impl NameKey {
    pub fn new(name: StringName) -> Self {
        Self(name)
    }

    // The interned name the key points to. Godot's StringName holds one
    // pointer to its interned data, and two names are equal exactly when
    // they hold the same pointer.
    fn identity(&self) -> usize {
        // SAFETY: the StringName's opaque storage is that one pointer, and
        // `self` keeps it alive while it is read.
        unsafe { *self.0.string_sys().cast::<usize>() }
    }
}

impl PartialEq for NameKey {
    fn eq(&self, other: &Self) -> bool {
        self.identity() == other.identity()
    }
}

impl Eq for NameKey {}

impl Hash for NameKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identity().hash(state);
    }
}

impl fmt::Display for NameKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
