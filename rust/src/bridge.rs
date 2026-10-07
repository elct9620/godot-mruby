//! What Ruby sees of the engine: the `Godot` module, a gem every game realm
//! opens with, and the values that cross between the engine and Ruby.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ptr;

use beni::{Error, Gem, Id, Mrb, Object, RClass, ReprValue, Symbol, Value, method, value::qnil};
use godot::builtin::{StringName, Variant};
use godot::classes::ClassDb;
use godot::obj::Singleton;
use godot::sys;

use crate::{compiler, log, realm};

mod bound_member;
mod bound_method;
mod name_key;
mod object;
mod ruby_object;
mod utility;
mod value;
mod value_type;

pub use bound_method::method_declarer;
pub use name_key::{NameKey, read_identity};
pub use object::{Owner, node_key};
pub use value::{ToEngine, ToRuby};

const FILE: &std::ffi::CStr = c"godot_mruby/bridge/godot.rb";

pub struct Godot;

impl Gem for Godot {
    fn init(mrb: &Mrb) -> Result<(), Error> {
        let godot = mrb.define_module(c"Godot")?;
        godot.define_singleton_method(
            mrb,
            c"__engine_superclass__",
            method!(engine_superclass, 1),
        )?;
        godot.define_singleton_method(mrb, c"__utilities__", method!(utility::utilities, 0))?;
        godot.define_singleton_method(mrb, c"__utility__", method!(utility::utility, -1))?;
        godot.define_singleton_method(
            mrb,
            c"__apply_utility__",
            method!(utility::apply_utility, 2),
        )?;
        object::define(mrb, godot)?;
        value_type::define(mrb, godot)?;
        compiler::run(
            mrb,
            FILE,
            include_str!("bridge/godot.rb"),
            log::compiler_warnings(FILE),
        )
    }
}

/// What the bridge keeps for a realm, in one place so a call reaches all of
/// it with one lookup and hands it on to what it calls.
#[derive(Default)]
struct BridgeData {
    // Godot::Object.
    object_class: Cell<Option<RClass>>,
    // The engine's name for each symbol the realm has asked the engine about.
    engine_names: RefCell<HashMap<Id, StringName>>,
    // The class under Godot of each engine class the realm has handed Ruby an
    // object of, by the engine's name for it.
    engine_classes: RefCell<HashMap<NameKey, RClass>>,
    // Godot::Value.
    value_class: Cell<Option<RClass>>,
    // The class under Godot of each value type the realm has handed Ruby a
    // value of, by the type's ordinal.
    value_classes: RefCell<Vec<Option<RClass>>>,
    // The nameless classes of bound methods and of bound members.
    bound_class: Cell<Option<RClass>>,
    bound_member_class: Cell<Option<RClass>>,
}

// What the bridge keeps for the realm `mrb` belongs to.
fn data(mrb: &Mrb) -> &BridgeData {
    realm::extension_data::<BridgeData>(mrb)
}

// The class `kept` holds, or the one `find` finds, kept there for the realm
// and rooted for the collector so it lives while it is kept.
fn find_class_once(
    mrb: &Mrb,
    kept: &Cell<Option<RClass>>,
    find: impl FnOnce() -> RClass,
) -> RClass {
    kept.get().unwrap_or_else(|| {
        let class = find();
        mrb.gc_register_forever(class.as_value());
        kept.set(Some(class));
        class
    })
}

// Runs an engine call that writes its answer into an uninitialized variant
// and reports failure through a call error, and answers either.
fn run_engine_call(
    call: impl FnOnce(sys::GDExtensionUninitializedVariantPtr, *mut sys::GDExtensionCallError),
) -> Result<Variant, sys::GDExtensionCallError> {
    // SAFETY: the interface is initialized while the extension runs, and an
    // engine call writes the answer whenever it leaves the error at CALL_OK.
    unsafe {
        Variant::new_with_var_uninit_result(|answer| {
            let mut error = sys::default_call_error();
            call(answer, ptr::addr_of_mut!(error));
            if error.error == sys::GDEXTENSION_CALL_OK {
                Ok(())
            } else {
                Err(error)
            }
        })
    }
}

/// Whether the engine class named `class` is a node class, the only kind a
/// node script extends.
pub fn is_node_class(class: &str) -> bool {
    ClassDb::singleton().is_parent_class(class, "Node")
}

// Godot.__engine_superclass__(name): the name of the engine class `name`
// inherits from, empty for the root, or nil when the engine has no such class.
fn engine_superclass(mrb: &Mrb, _godot: Value, name: Symbol) -> Value {
    let class_db = ClassDb::singleton();
    let Some(name) = name.name(mrb).filter(|name| class_db.class_exists(name)) else {
        return qnil().as_value();
    };
    let parent = class_db.get_parent_class(&name).to_string();
    mrb.str_new(parent.as_bytes()).as_value()
}
