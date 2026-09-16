//! What Ruby sees of the engine: the `Godot` module, a gem every game realm
//! opens with.

use beni::{Error, Gem, Mrb, Object, Symbol, Value, method};
use godot::classes::ClassDb;
use godot::obj::Singleton;

use crate::{compiler, log};

const FILE: &std::ffi::CStr = c"godot_mruby/bridge/godot.rb";

pub struct Godot;

impl Gem for Godot {
    fn init(mrb: &Mrb) -> Result<(), Error> {
        mrb.define_module(c"Godot")?.define_singleton_method(
            mrb,
            c"__engine_superclass__",
            method!(engine_superclass, 1),
        )?;
        compiler::run(
            mrb,
            FILE,
            include_str!("bridge/godot.rb"),
            log::compiler_warnings(FILE),
        )
    }
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
