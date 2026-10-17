//! What Ruby sees of the engine: the `Godot` module, a gem every game realm
//! opens with, and the values that cross between the engine and Ruby.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ptr;

use beni::typed_data::RTypedData;
use beni::{
    Class as _, Error, Gem, Id, Module, Mrb, Object, RClass, ReprValue, Symbol, TryConvert,
    TypedData, Value, method, value::qnil,
};
use godot::builtin::{StringName, Variant};
use godot::classes::ClassDb;
use godot::obj::Singleton;
use godot::sys;

use crate::{compiler, log, realm};

mod bound_member;
mod bound_method;
mod declaration;
mod object;
mod refusal;
mod ruby_object;
mod utility;
mod value;
mod value_type;

pub use bound_method::{find_method_declarer, is_string_name_member};
pub use object::{Owner, derive_object_key, release_collected};
pub use value::{EngineCrossing, RubyCrossing};
pub use value_type::collect_value_type_names;

const FILE: &std::ffi::CStr = c"godot_mruby/bridge/godot.rb";

pub struct Godot;

impl Gem for Godot {
    fn init(mrb: &Mrb) -> Result<(), Error> {
        let godot = mrb.define_module(c"Godot")?;
        godot.define_singleton_method(
            mrb,
            c"__engine_superclass_by_name__",
            method!(engine_superclass_by_name, 1),
        )?;
        godot.define_singleton_method(mrb, c"__utilities__", method!(utility::utilities, 0))?;
        godot.define_singleton_method(
            mrb,
            c"__call_utility__",
            method!(utility::call_utility, -1),
        )?;
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
            log::write_compiler_warnings(FILE),
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
    engine_classes: RefCell<HashMap<StringName, RClass>>,
    // Godot::Value.
    value_class: Cell<Option<RClass>>,
    // The class under Godot of each value type the realm has handed Ruby a
    // value of, by the type's ordinal.
    value_classes: RefCell<Vec<Option<RClass>>>,
    // The nameless classes of bound methods and of bound members.
    bound_method_class: Cell<Option<RClass>>,
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

// Whether `class` is `ancestor` or descends from it, as Ruby's `class <=
// ancestor` answers, read without running Ruby.
fn has_ancestor(mrb: &Mrb, class: RClass, ancestor: RClass) -> bool {
    let mut current = Some(class);
    while let Some(class) = current {
        if class.as_value().is_equal(mrb, ancestor.as_value()) {
            return true;
        }
        current = class.superclass(mrb);
    }
    false
}

// The copy Ruby's dup or clone made of `original`, given what `make_copy`
// makes of what `original` carries, as a Ruby extension's data fills its
// copy in initialize_copy; or the TypeError raised when the two differ in
// class, `make_copy` refuses what `original` carries, or the copy already
// carries one.
fn fill_copy<T: TypedData>(
    mrb: &Mrb,
    copy: RTypedData,
    original: Value,
    make_copy: impl FnOnce(RClass, &T) -> Result<T, Error>,
) -> Result<RTypedData, Error> {
    if copy.as_value().is_equal(mrb, original) {
        return Ok(copy);
    }
    let class = copy.as_value().class(mrb);
    let same_class = original
        .class(mrb)
        .as_value()
        .is_equal(mrb, class.as_value());
    if !same_class {
        let message = "initialize_copy should take same class object";
        return Err(refusal::make_type_error(mrb, message));
    }
    let carried = make_copy(class, <&T>::try_convert(original, mrb)?)?;
    copy.init(mrb, carried).map(|()| copy).map_err(|_| {
        let path = class.path(mrb).unwrap_or_default();
        refusal::make_type_error(mrb, &format!("{path} is already initialized"))
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

// Godot.__engine_superclass_by_name__(name): the name of the engine class `name`
// inherits from, empty for the root, or nil when the engine has no such class.
fn engine_superclass_by_name(mrb: &Mrb, _godot: Value, name: Symbol) -> Value {
    let class_db = ClassDb::singleton();
    let Some(name) = name.name(mrb).filter(|name| class_db.class_exists(name)) else {
        return qnil().as_value();
    };
    let parent = class_db.get_parent_class(&name).to_string();
    mrb.str_new(parent.as_bytes()).as_value()
}

// The engine class a class under Godot stands for, by the path Ruby names
// it with; a path outside Godot is its own name.
fn read_engine_name(path: &str) -> &str {
    path.strip_prefix("Godot::").unwrap_or(path)
}

// The file the realm's class index names for the constant `path` spells.
fn file_by_class_path(mrb: &Mrb, path: &str) -> Option<String> {
    let names: Vec<String> = path.split("::").map(str::to_owned).collect();
    realm::file_by_constant(mrb, &names)
}
