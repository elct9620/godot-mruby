//! The methods a class has of its own: those it defines and those of the
//! modules it takes in. A module prepended to `Module` tells the realm when
//! one of them gains or loses a method, or takes in a module, after its
//! class's file has run, so the class's methods are read and published again.

use std::sync::Arc;

use beni::{
    Error, FromValue, IntoValue, Module, Mrb, RArray, ReprValue, Symbol, TryConvert, Value, method,
    value::qnil,
};

use super::{bookkeeping, compile, constants, executor, key_by_path};
use crate::snapshot;

pub(super) fn define(mrb: &Mrb) -> Result<(), Error> {
    let module = mrb.class_get(c"Module")?;
    module.define_private_method(mrb, c"__record_change__", method!(record_change, 1))?;
    compile(mrb, c"godot_mruby/methods.rb", include_str!("methods.rb"))
}

/// The names of the methods the class the file at `path` names has of its
/// own: those it defines and those of the modules it takes in, up to its
/// superclass. The class is noted as a taker of each of those, so a method
/// added to one later is read again. None when the file named no class.
pub(super) fn methods_by_path(mrb: &Mrb, path: &str) -> Vec<String> {
    let Some(class) = constants::constant_by_key(mrb, &key_by_path(mrb, path)) else {
        return Vec::new();
    };
    let Some(taken) = class
        .funcall(mrb, c"__taken_in__", &[])
        .ok()
        .and_then(RArray::from_value)
    else {
        return Vec::new();
    };
    let own = [false.into_value(mrb)];
    let mut names: Vec<String> = Vec::new();
    for module in taken.entries(mrb) {
        if let Some(id) = read_id(mrb, module) {
            record_taker(mrb, id, path);
        }
        let methods = module
            .funcall(mrb, c"instance_methods", &own)
            .ok()
            .and_then(RArray::from_value);
        let found = methods.into_iter().flat_map(|methods| methods.entries(mrb));
        for name in found
            .filter_map(Symbol::from_value)
            .filter_map(|name| name.name(mrb))
        {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

/// Publishes the methods of each class Ruby changed the methods of. Each is
/// read before the snapshot is borrowed, since reading runs Ruby.
pub(super) fn publish_changed(mrb: &Mrb) {
    let bookkeeping = bookkeeping(mrb);
    let paths = std::mem::take(&mut *bookkeeping.changed.borrow_mut());
    if paths.is_empty() {
        return;
    }
    let read: Vec<_> = paths
        .into_iter()
        .map(|path| {
            let methods = methods_by_path(mrb, &path);
            (path, methods)
        })
        .collect();
    let mut draft = bookkeeping.snapshot.borrow_mut();
    for (path, methods) in read {
        Arc::make_mut(&mut draft).record_methods(&path, methods);
    }
    snapshot::publish(Arc::clone(&draft));
}

// Module#__record_change__(module): `module` has just gained or lost a
// method, or taken in a module, which changes the methods of every class
// taking it in. Outside a file's run they are published now; a run
// publishes them as it ends, so no thread reads a class half run.
fn record_change(mrb: &Mrb, _receiver: Value, module: Value) -> Value {
    let bookkeeping = bookkeeping(mrb);
    let takers = read_id(mrb, module).and_then(|id| bookkeeping.takers.borrow().get(&id).cloned());
    if let Some(takers) = takers {
        bookkeeping.changed.borrow_mut().extend(takers);
    }
    if !executor::is_file_running(mrb) {
        publish_changed(mrb);
    }
    qnil().as_value()
}

// Notes that the class of the file at `path` takes in the module `id` names.
fn record_taker(mrb: &Mrb, id: i64, path: &str) {
    bookkeeping(mrb)
        .takers
        .borrow_mut()
        .entry(id)
        .or_default()
        .insert(path.to_owned());
}

// The identity of `module`, which no class overriding `hash` changes.
fn read_id(mrb: &Mrb, module: Value) -> Option<i64> {
    let id = module.funcall(mrb, c"__id__", &[]).ok()?;
    i64::try_convert(id, mrb).ok()
}
