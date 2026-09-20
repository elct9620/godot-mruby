//! A node's instance of a `RubyScript`, handed to Godot through the
//! extension interface directly rather than gdext's `ScriptInstance`: Godot
//! deletes an instance the moment its node's script is set, even while a
//! call into it is running, and gdext keeps the instance borrowed for the
//! whole call. Each callback here borrows the instance only until Ruby is
//! about to run, and touches nothing of it afterwards, as GDScript, C# and
//! other languages' instances do.

use std::ffi::c_void;
use std::sync::{Arc, Mutex, PoisonError};

use godot::classes::{Object, Script, ScriptLanguage};
use godot::meta::conv::RawPtr;
use godot::prelude::*;
use godot::sys;

use crate::ancestry::Ancestry;
use crate::bridge::{self, Owner, ToEngine, ToRuby};
use crate::error;
use crate::log::GodotLog;
use crate::parser::Header;
use crate::realm::{self, Built, Key, RubyError};
use crate::snapshot;

/// A node's instance of a `RubyScript`. It holds no Ruby value: the node's
/// Ruby object is built in the game's realm the first time Godot calls a
/// method its class defines, unless Ruby made the node, and the realm holds
/// it under the node's key. Nothing in it changes but its stage, so Godot
/// may ask it from any thread.
pub struct RubyInstance {
    script: Gd<Script>,
    path: String,
    owner: InstanceId,
    // The header and ancestry its script had as the instance was made, which
    // answer which methods the node's class has without entering the realm.
    header: Arc<Header>,
    ancestry: Arc<Ancestry>,
    // The language its script reports, which Godot also asks the instance for.
    language: Gd<ScriptLanguage>,
    // What the node prints as while its script says nothing about it.
    display: GString,
    // Shared with the calls running Ruby, which outlive the instance when
    // Ruby takes its node's script away.
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
            path: script.get_path().to_string(),
            script,
            owner: owner.instance_id(),
            header,
            ancestry,
            language,
            display: GString::from(&owner.to_string()),
            stage: Arc::new(Mutex::new(Stage::Recorded)),
        }
    }

    /// Hands the instance to Godot for the node it was made for, which frees
    /// it with the node or when the node's script is set.
    pub fn into_godot(self) -> RawPtr<*mut c_void> {
        let data = Box::into_raw(Box::new(self));
        // SAFETY: the interface is initialized while the extension runs;
        // `INFO` is static, and Godot hands `data` back to each callback
        // until `free` takes it.
        unsafe {
            let created = sys::interface_fn!(script_instance_create3)(&INFO, data.cast());
            RawPtr::new(created.cast::<c_void>())
        }
    }

    // Whether the node's class defines `method` or inherits it from a file.
    fn has(&self, method: &str) -> bool {
        self.header.has_method(method)
            || snapshot::latest().has_method(&self.path, method)
            || self.ancestry.has_method(method)
    }

    // What a call into Ruby needs of the instance, taken before Ruby runs.
    fn caller(&self) -> Caller {
        Caller {
            path: self.path.clone(),
            owner: self.owner,
            stage: Arc::clone(&self.stage),
        }
    }
}

// Godot frees an instance with its node, on whatever thread frees the node,
// or when the node's script is set, even while a call into it runs.
impl Drop for RubyInstance {
    fn drop(&mut self) {
        if let Stage::Built(key) = *self.stage.lock().unwrap_or_else(PoisonError::into_inner) {
            realm::release(key);
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

// What Godot calls on a Ruby instance. A callback not given leaves Godot's
// default: no properties of the script's own, no fallback, nothing to
// revert; properties wait for exports.
static INFO: sys::GDExtensionScriptInstanceInfo3 = sys::GDExtensionScriptInstanceInfo3 {
    set_func: None,
    get_func: None,
    get_property_list_func: None,
    free_property_list_func: None,
    get_class_category_func: None,
    property_can_revert_func: None,
    property_get_revert_func: None,
    get_owner_func: None,
    get_property_state_func: None,
    get_method_list_func: None,
    free_method_list_func: None,
    get_property_type_func: None,
    validate_property_func: None,
    has_method_func: Some(has_method),
    get_method_argument_count_func: None,
    call_func: Some(call),
    notification_func: Some(notification),
    to_string_func: Some(to_string),
    refcount_incremented_func: None,
    refcount_decremented_func: Some(refcount_decremented),
    get_script_func: Some(get_script),
    is_placeholder_func: None,
    set_fallback_func: None,
    get_fallback_func: None,
    get_language_func: Some(get_language),
    free_func: Some(free),
};

// The instance Godot hands a callback, shared by callbacks on several
// threads, since nothing changes it but its stage, behind a mutex.
//
// SAFETY: `data` is what `into_godot` handed Godot, alive until `free`; the
// reference must not be used once Ruby runs, since Ruby may have Godot free
// the instance.
unsafe fn instance<'a>(data: sys::GDExtensionScriptInstanceDataPtr) -> &'a RubyInstance {
    unsafe { &*data.cast::<RubyInstance>() }
}

// SAFETY: `method` is a StringName Godot hands for the call, which
// `StringName` lays out as.
unsafe fn name(method: sys::GDExtensionConstStringNamePtr) -> String {
    unsafe { &*method.cast::<StringName>() }.to_string()
}

unsafe extern "C" fn has_method(
    data: sys::GDExtensionScriptInstanceDataPtr,
    method: sys::GDExtensionConstStringNamePtr,
) -> sys::GDExtensionBool {
    // SAFETY: the instance lives for this call, which runs no Ruby.
    let has = unsafe { instance(data).has(&name(method)) };
    sys::GDExtensionBool::from(has)
}

unsafe extern "C" fn call(
    data: sys::GDExtensionScriptInstanceDataPtr,
    method: sys::GDExtensionConstStringNamePtr,
    args: *const sys::GDExtensionConstVariantPtr,
    count: sys::GDExtensionInt,
    answer: sys::GDExtensionVariantPtr,
    error: *mut sys::GDExtensionCallError,
) {
    // SAFETY: the method name lives for the call.
    let method = unsafe { name(method) };
    // SAFETY: the instance lives until Ruby runs, and is not used after.
    let caller = {
        let instance = unsafe { instance(data) };
        if !instance.has(&method) {
            // SAFETY: Godot hands an error to fill.
            unsafe { (*error).error = sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD };
            return;
        }
        instance.caller()
    };
    let args: &[&Variant] = if args.is_null() {
        &[]
    } else {
        // SAFETY: Godot hands `count` live variants, which `Variant` lays out
        // as, alive for the call.
        unsafe { std::slice::from_raw_parts(args.cast::<&Variant>(), count as usize) }
    };
    let answered = caller.send(&method, args);
    // SAFETY: Godot hands an initialized variant and an error to fill, both
    // its own, so they outlive the instance.
    unsafe {
        *answer.cast::<Variant>() = answered;
        (*error).error = sys::GDEXTENSION_CALL_OK;
    }
}

unsafe extern "C" fn notification(
    data: sys::GDExtensionScriptInstanceDataPtr,
    what: i32,
    _reversed: sys::GDExtensionBool,
) {
    // SAFETY: the instance lives until Ruby runs, and is not used after.
    let caller = {
        let instance = unsafe { instance(data) };
        if !instance.has("_notification") {
            return;
        }
        instance.caller()
    };
    caller.send("_notification", &[&what.to_variant()]);
}

unsafe extern "C" fn to_string(
    data: sys::GDExtensionScriptInstanceDataPtr,
    valid: *mut sys::GDExtensionBool,
    out: sys::GDExtensionStringPtr,
) {
    // SAFETY: the instance lives for this call, which runs no Ruby; Godot
    // hands an initialized String, which `GString` lays out as, and a flag.
    unsafe {
        *out.cast::<GString>() = instance(data).display.clone();
        *valid = sys::GDExtensionBool::from(true);
    }
}

// The node may be freed once nothing else refers to it: its Ruby object
// holds it through its key, not a reference.
unsafe extern "C" fn refcount_decremented(
    _data: sys::GDExtensionScriptInstanceDataPtr,
) -> sys::GDExtensionBool {
    sys::GDExtensionBool::from(true)
}

unsafe extern "C" fn get_script(
    data: sys::GDExtensionScriptInstanceDataPtr,
) -> sys::GDExtensionObjectPtr {
    // SAFETY: the instance lives for this call; Godot takes its own reference.
    unsafe { instance(data).script.obj_sys() }
}

unsafe extern "C" fn get_language(
    data: sys::GDExtensionScriptInstanceDataPtr,
) -> sys::GDExtensionScriptLanguagePtr {
    // SAFETY: the instance lives for this call.
    unsafe { instance(data).language.obj_sys().cast() }
}

unsafe extern "C" fn free(data: sys::GDExtensionScriptInstanceDataPtr) {
    // SAFETY: Godot frees an instance once, handing back what `into_godot`
    // boxed; a call still running holds only its `Caller`.
    drop(unsafe { Box::from_raw(data.cast::<RubyInstance>()) });
}
