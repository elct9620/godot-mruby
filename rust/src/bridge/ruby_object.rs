//! Ruby's own objects as the engine holds them: a Proc or Method as a
//! Callable running it, and any other object as a `RubyObject`. The engine
//! keeps only the key the realm holds the object under, so no Ruby object is
//! reachable from anywhere but its own realm, and the realm lets go of it once
//! the engine drops the last reference.

use godot::builtin::{Callable, GString, Variant};
use godot::classes::{IRefCounted, RefCounted};
use godot::obj::{Base, Gd};
use godot::prelude::{GodotClass, godot_api};

use super::value::{ToEngine, ToRuby};
use crate::error;
use crate::log::GodotLog;
use crate::realm::{self, Key};

/// A key the engine keeps for a Ruby object, let go of when the engine lets
/// go of it, on whatever thread that is.
struct Kept(Key);

impl Drop for Kept {
    fn drop(&mut self) {
        realm::release(self.0);
    }
}

/// A Callable calling `call` on the Proc or Method `key` holds.
pub fn callable(key: Key, name: String) -> Callable {
    let kept = Kept(key);
    Callable::from_sync_fn(name, move |args: &[&Variant]| call(&kept, args))
}

fn call(kept: &Kept, args: &[&Variant]) -> Variant {
    let checked = args.iter().map(|arg| ToRuby::checked(arg));
    let args = match checked.collect::<Result<Vec<_>, _>>() {
        Ok(args) => args,
        Err(reason) => {
            error!("A Ruby callable was not called: {reason}");
            return Variant::nil();
        }
    };
    realm::enter(|realm| realm.send::<_, ToEngine>(kept.0, "call", args))
        .map(|ToEngine(answer)| answer)
        .unwrap_or_else(|failed| {
            failed.write(&GodotLog);
            Variant::nil()
        })
}

/// A Ruby object as the engine holds it, standing for an object no engine
/// type fits.
#[derive(GodotClass)]
#[class(base = RefCounted, no_init)]
pub struct RubyObject {
    kept: Kept,
    // What the engine prints it as: the Ruby object's class.
    class: GString,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for RubyObject {
    fn to_string(&self) -> GString {
        GString::from(&format!("<RubyObject:{}>", self.class))
    }
}

impl RubyObject {
    /// A new `RubyObject` for the object of class `class` that `key` holds.
    pub fn holding(key: Key, class: &str) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            kept: Kept(key),
            class: GString::from(class),
            base,
        })
    }

    /// The key the realm holds the Ruby object under.
    pub fn key(&self) -> Key {
        self.kept.0
    }
}
