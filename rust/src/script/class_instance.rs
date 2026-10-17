//! The script instance a Ruby script carries for its class, as a GDScript
//! answers calls made on itself: `new`, and the methods the class defines on
//! `self`, run on the class in the game's realm. Godot sends every call on
//! the script through it before the script's own methods, so it answers
//! nothing else and never enters the realm for it.

use godot::builtin::{GString, StringName, Variant};
use godot::meta::ToGodot;
use godot::obj::{Gd, InstanceId};
use godot::sys;
use smallvec::SmallVec;

use super::RubyScript;
use crate::bridge::{EngineCrossing, RubyCrossing};
use crate::language;
use crate::log::{GodotLog, catch_panic};
use crate::realm::{self, RubyError};

// What the class's instance knows of its script: only which one it is, so
// it reads the script as it is now, after any reload.
struct ClassInstance {
    script: InstanceId,
}

/// Gives `script` its class's instance, which Godot frees with the script.
pub fn attach(script: &Gd<RubyScript>) {
    let data = Box::into_raw(Box::new(ClassInstance {
        script: script.instance_id(),
    }));
    // SAFETY: the interface is initialized while the extension runs; `INFO`
    // is static, and Godot hands `data` back to each callback until `free`
    // takes it, which it does as it frees the script.
    unsafe {
        let instance = sys::interface_fn!(script_instance_create3)(&INFO, data.cast());
        sys::interface_fn!(object_set_script_instance)(script.obj_sys(), instance.cast());
    }
}

// What calling `method` on the class of `script` with `args` answers, or
// none when the class answers no such call: `new` makes an object of it,
// and a method it defines on `self` runs. An exception is reported and
// answers null, as a method that returned nothing does.
fn call_on_class(script: &Gd<RubyScript>, method: &str, args: &[&Variant]) -> Option<Variant> {
    let constant = {
        let script = script.bind();
        (method == "new" || script.has_class_method(method)).then(|| script.class_constant())
    }?;
    let named: [Variant; 2] = [
        GString::from(&constant).to_variant(),
        StringName::from(method).to_variant(),
    ];
    let args: SmallVec<[_; 6]> = named
        .iter()
        .chain(args.iter().copied())
        .map(RubyCrossing)
        .collect();
    let answer =
        realm::enter(|realm| realm.call::<_, EngineCrossing>("Godot", c"__call_class__", args))
            .map(|EngineCrossing(answer)| answer)
            .unwrap_or_else(|failed: RubyError| {
                failed.write(&GodotLog);
                Variant::nil()
            });
    Some(answer)
}

// The script the instance was attached to, while it lives.
fn find_script(data: sys::GDExtensionScriptInstanceDataPtr) -> Option<Gd<RubyScript>> {
    // SAFETY: `data` is what `attach` boxed, alive until `free`.
    let id = unsafe { &*data.cast::<ClassInstance>() }.script;
    Gd::try_from_instance_id(id).ok()
}

unsafe extern "C" fn has_method(
    data: sys::GDExtensionScriptInstanceDataPtr,
    method: sys::GDExtensionConstStringNamePtr,
) -> sys::GDExtensionBool {
    catch_panic("has_method", sys::GDExtensionBool::from(false), || {
        // SAFETY: Godot hands a method name living for the call.
        let method = unsafe { &*method.cast::<StringName>() }.to_string();
        let answers = method == "new"
            || find_script(data).is_some_and(|script| script.bind().has_class_method(&method));
        sys::GDExtensionBool::from(answers)
    })
}

unsafe extern "C" fn call(
    data: sys::GDExtensionScriptInstanceDataPtr,
    method: sys::GDExtensionConstStringNamePtr,
    args: *const sys::GDExtensionConstVariantPtr,
    count: sys::GDExtensionInt,
    answer: sys::GDExtensionVariantPtr,
    error: *mut sys::GDExtensionCallError,
) {
    // SAFETY: Godot hands a method name living for the call.
    let method = unsafe { &*method.cast::<StringName>() }.to_string();
    let args: &[&Variant] = if args.is_null() {
        &[]
    } else {
        // SAFETY: Godot hands `count` live variants, which `Variant` lays
        // out as, alive for the call.
        unsafe { std::slice::from_raw_parts(args.cast::<&Variant>(), count as usize) }
    };
    let answered = catch_panic("call", None, || {
        find_script(data).and_then(|script| call_on_class(&script, &method, args))
    });
    // SAFETY: Godot hands an initialized variant and an error to fill.
    unsafe {
        match answered {
            Some(answered) => {
                *answer.cast::<Variant>() = answered;
                (*error).error = sys::GDEXTENSION_CALL_OK;
            }
            None => (*error).error = sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD,
        }
    }
}

// The script may be freed once nothing refers to it: an extension's
// instance that says nothing keeps its object alive for good.
unsafe extern "C" fn refcount_decremented(
    _data: sys::GDExtensionScriptInstanceDataPtr,
) -> sys::GDExtensionBool {
    sys::GDExtensionBool::from(true)
}

unsafe extern "C" fn get_language(
    _data: sys::GDExtensionScriptInstanceDataPtr,
) -> sys::GDExtensionScriptLanguagePtr {
    language::language().map_or(std::ptr::null_mut(), |language| language.obj_sys().cast())
}

unsafe extern "C" fn free(data: sys::GDExtensionScriptInstanceDataPtr) {
    // SAFETY: Godot frees an instance once, handing back what `attach` boxed.
    drop(unsafe { Box::from_raw(data.cast::<ClassInstance>()) });
}

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
    notification_func: None,
    to_string_func: None,
    refcount_incremented_func: None,
    refcount_decremented_func: Some(refcount_decremented),
    get_script_func: None,
    is_placeholder_func: None,
    set_fallback_func: None,
    get_fallback_func: None,
    get_language_func: Some(get_language),
    free_func: Some(free),
};
