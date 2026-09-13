use godot::classes::{Object, Script, ScriptLanguage};
use godot::meta::error::CallErrorType;
use godot::obj::script::{ScriptInstance, SiMut};
use godot::prelude::*;
use godot::register::info::{MethodInfo, PropertyInfo};

use crate::{interpreter, language};

/// A node's instance of a `RubyScript`. It holds no Ruby state: every entry
/// runs the script's file in the game's interpreter if it has not run yet,
/// and otherwise leaves the node's own members to answer.
pub struct RubyInstance {
    script: Gd<Script>,
    // What the node prints as while its script says nothing about it.
    display: GString,
}

impl RubyInstance {
    pub fn new(script: Gd<Script>, owner: &Gd<Object>) -> Self {
        Self {
            script,
            display: GString::from(&owner.to_string()),
        }
    }

    fn enter(&self) {
        let path = self.script.get_path().to_string();
        let source = || self.script.get_source_code().to_string();
        if let Err(message) = interpreter::run_once(&path, source) {
            godot_error!("{message}");
        }
    }
}

impl ScriptInstance for RubyInstance {
    type Base = Object;

    fn class_name(&self) -> GString {
        self.script.get_class()
    }

    fn set_property(this: SiMut<Self>, _name: StringName, _value: &Variant) -> bool {
        this.enter();
        false
    }

    fn get_property(&self, _name: StringName) -> Option<Variant> {
        self.enter();
        None
    }

    fn get_property_list(&self) -> Vec<PropertyInfo> {
        Vec::new()
    }

    fn get_method_list(&self) -> Vec<MethodInfo> {
        Vec::new()
    }

    fn call(
        this: SiMut<Self>,
        _method: StringName,
        _args: &[&Variant],
    ) -> Result<Variant, CallErrorType> {
        this.enter();
        Err(CallErrorType::InvalidMethod)
    }

    fn on_notification(this: SiMut<Self>, _what: i32, _reversed: bool) {
        this.enter();
    }

    fn is_placeholder(&self) -> bool {
        false
    }

    fn has_method(&self, _method: StringName) -> bool {
        false
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
        language::registered()
            .expect("the Ruby language outlives every Ruby script instance")
            .upcast()
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
