//! What Ruby sees of the engine: the `Godot` module, a gem every game realm
//! opens with, and the values that cross between the engine and Ruby.

use std::cell::Cell;

use beni::{Error, Gem, Mrb, Object, RClass, ReprValue, Symbol, Value, method};
use godot::classes::ClassDb;
use godot::obj::Singleton;

use crate::{compiler, log};

mod name_key;
mod object;
mod ruby_object;
mod utility;
mod value;
mod value_type;

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
        godot.define_singleton_method(mrb, c"__utility__", method!(utility::utility, 2))?;
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
        return Value::nil();
    };
    let parent = class_db.get_parent_class(&name).to_string();
    mrb.str_new(parent.as_bytes()).as_value()
}
