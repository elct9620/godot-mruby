use std::sync::{Arc, Mutex, PoisonError};

use godot::classes::{Object, Script, ScriptLanguage};
use godot::meta::error::CallErrorType;
use godot::obj::script::{ScriptInstance, SiMut};
use godot::prelude::*;
use godot::register::info::{MethodInfo, PropertyInfo};

use crate::ancestry::Ancestry;
use crate::bridge::{self, Owner, Running, ToEngine, ToRuby};
use crate::error;
use crate::log::GodotLog;
use crate::parser::Header;
use crate::realm::{self, Built, Key, RubyError};

/// A node's instance of a `RubyScript`. It holds no Ruby value: the node's
/// Ruby object is built in the game's realm the first time Godot calls a
/// method its class defines, unless Ruby made the node, and the realm holds
/// it under the node's key.
pub struct RubyInstance {
    script: Gd<Script>,
    owner: InstanceId,
    // The header and ancestry its script had as the instance was made, which
    // answer which methods the node's class has without entering the realm.
    header: Arc<Header>,
    ancestry: Arc<Ancestry>,
    // The language its script reports, which Godot also asks the instance for.
    language: Gd<ScriptLanguage>,
    // What the node prints as while its script says nothing about it.
    display: GString,
    // Shared with the calls running Ruby, which give the instance up while
    // Ruby runs, since the engine may call back into it or free it.
    stage: Arc<Mutex<Stage>>,
}

/// How far a node's Ruby object has come.
#[derive(Clone, Copy)]
enum Stage {
    /// Not built yet: making a node runs no Ruby.
    Recorded,
    Built(Key),
    /// Its file or `initialize` raised, which was reported; the node calls
    /// nothing from then on, as an engine-made object is never built again.
    Failed,
}

impl RubyInstance {
    pub fn new(
        script: Gd<Script>,
        header: Arc<Header>,
        ancestry: Arc<Ancestry>,
        language: Gd<ScriptLanguage>,
        owner: &Gd<Object>,
    ) -> Self {
        Self {
            script,
            owner: owner.instance_id(),
            header,
            ancestry,
            language,
            display: GString::from(&owner.to_string()),
            stage: Arc::new(Mutex::new(Stage::Recorded)),
        }
    }

    // Whether the node's class defines `method` or inherits it from a file.
    fn has(&self, method: &str) -> bool {
        self.header.has_method(method) || self.ancestry.has_method(method)
    }

    // What a call into Ruby needs of the instance, taken before the instance
    // is given up.
    fn caller(&self) -> Caller {
        Caller {
            path: self.script.get_path().to_string(),
            owner: self.owner,
            stage: Arc::clone(&self.stage),
        }
    }
}

/// What calls a method on a node's Ruby object, holding nothing of the
/// instance itself, so the engine may call back into the instance or free it
/// while Ruby runs.
struct Caller {
    path: String,
    owner: InstanceId,
    stage: Arc<Mutex<Stage>>,
}

impl Caller {
    fn stage(&self) -> Stage {
        *self.stage.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn settle(&self, stage: Stage) {
        *self.stage.lock().unwrap_or_else(PoisonError::into_inner) = stage;
    }

    // The node's Ruby object, built at the first call that needs it and
    // initialized once it is held, unless Ruby made the node and built it.
    // A call arriving while it initializes finds it held; one arriving while
    // its file runs, before the class exists, finds none and builds nothing.
    fn object(&self) -> Option<Key> {
        match self.stage() {
            Stage::Built(key) => return Some(key),
            Stage::Failed => return None,
            Stage::Recorded => {}
        }
        let key = bridge::node_key(self.owner);
        let owner = [Owner(self.owner)];
        let built = realm::enter(|realm| realm.build(&self.path, key, c"__allocate__", owner))
            .and_then(|built| {
                if built == Built::Waiting {
                    return Ok(None);
                }
                self.settle(Stage::Built(key));
                if built == Built::Made {
                    realm::enter(|realm| realm.send::<ToRuby, ToEngine>(key, "initialize", []))?;
                }
                Ok(Some(key))
            });
        built
            .inspect_err(|failed| {
                failed.write(&GodotLog);
                realm::release(key);
                self.settle(Stage::Failed);
            })
            .ok()
            .flatten()
    }

    // Calls `method` on the node's Ruby object; an exception, or an argument
    // that cannot reach Ruby, is reported and answers null, as a callback
    // that returned nothing does.
    fn send(&self, method: &str, args: &[&Variant]) -> Variant {
        let Some(key) = self.object() else {
            return Variant::nil();
        };
        let checked = args.iter().map(|arg| ToRuby::checked(arg));
        let args = match checked.collect::<Result<Vec<_>, _>>() {
            Ok(args) => args,
            Err(reason) => {
                error!("#{method} was not called: {reason}");
                return Variant::nil();
            }
        };
        realm::enter(|realm| realm.send::<_, ToEngine>(key, method, args))
            .map(|ToEngine(answer)| answer)
            .unwrap_or_else(|failed: RubyError| {
                failed.write(&GodotLog);
                Variant::nil()
            })
    }
}

// Godot frees an instance with its node, on whatever thread frees the node.
impl Drop for RubyInstance {
    fn drop(&mut self) {
        if let Stage::Built(key) = *self.stage.lock().unwrap_or_else(PoisonError::into_inner) {
            realm::release(key);
        }
    }
}

impl ScriptInstance for RubyInstance {
    type Base = Object;

    fn class_name(&self) -> GString {
        self.script.get_class()
    }

    fn set_property(_this: SiMut<Self>, _name: StringName, _value: &Variant) -> bool {
        false
    }

    fn get_property(&self, _name: StringName) -> Option<Variant> {
        None
    }

    fn get_property_list(&self) -> Vec<PropertyInfo> {
        Vec::new()
    }

    fn get_method_list(&self) -> Vec<MethodInfo> {
        Vec::new()
    }

    fn call(
        mut this: SiMut<Self>,
        method: StringName,
        args: &[&Variant],
    ) -> Result<Variant, CallErrorType> {
        let method = method.to_string();
        if !this.has(&method) {
            return Err(CallErrorType::InvalidMethod);
        }
        let caller = this.caller();
        let _given_up = this.base_mut();
        let _running = Running::start(caller.owner);
        Ok(caller.send(&method, args))
    }

    fn on_notification(mut this: SiMut<Self>, what: i32, _reversed: bool) {
        if this.has("_notification") {
            let caller = this.caller();
            let _given_up = this.base_mut();
            let _running = Running::start(caller.owner);
            caller.send("_notification", &[&what.to_variant()]);
        }
    }

    fn is_placeholder(&self) -> bool {
        false
    }

    fn has_method(&self, method: StringName) -> bool {
        self.has(&method.to_string())
    }

    fn get_script(&self) -> &Gd<Script> {
        &self.script
    }

    fn get_property_type(&self, _name: StringName) -> VariantType {
        VariantType::NIL
    }

    fn to_string(&self) -> GString {
        self.display.clone()
    }

    fn get_property_state(&self) -> Vec<(StringName, Variant)> {
        Vec::new()
    }

    fn get_language(&self) -> Gd<ScriptLanguage> {
        self.language.clone()
    }

    fn on_refcount_decremented(&self) -> bool {
        true
    }

    fn on_refcount_incremented(&self) {}

    fn property_get_fallback(&self, _name: StringName) -> Option<Variant> {
        None
    }

    fn property_set_fallback(_this: SiMut<Self>, _name: StringName, _value: &Variant) -> bool {
        false
    }

    fn get_method_argument_count(&self, _method: StringName) -> Option<u32> {
        None
    }
}
