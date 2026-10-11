//! The methods a class has of its own: those it defines and those of the
//! modules it takes in. A module prepended to `Module` tells the realm when
//! one of them gains a method or a module after its class's file has run, so
//! the class's methods are read and published again.

use std::collections::BTreeSet;
use std::sync::Arc;

use beni::{
    Error, FromValue, IntoValue, Module, Mrb, RArray, ReprValue, Symbol, TryConvert, Value, method,
    value::qnil,
};

use super::{bookkeeping, compile, constants, executor, key_by_path};
use crate::snapshot::{self, Snapshot};

pub(super) fn define(mrb: &Mrb) -> Result<(), Error> {
    let module = mrb.class_get(c"Module")?;
    module.define_private_method(mrb, c"__record_change__", method!(record_change, 1))?;
    compile(mrb, c"godot_mruby/methods.rb", include_str!("methods.rb"))
}

/// The names of the methods the class the file at `path` names has of its
/// own: those it defines and those of the modules it takes in, up to its
/// superclass. Each of those is noted as taken in by the class, so a method
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
        record_taker(mrb, module, path);
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

/// Takes into `draft` the methods of each class Ruby changed the methods of.
pub(super) fn record_changed_methods(mrb: &Mrb, draft: &mut Arc<Snapshot>) {
    let paths = std::mem::take(&mut *bookkeeping(mrb).changed.borrow_mut());
    for path in paths {
        let methods = methods_by_path(mrb, &path);
        Arc::make_mut(draft).record_methods(&path, methods);
    }
}

// Module#__record_change__(module): `module` has just gained a method or a
// module, which changes the methods of every class taking it in. Outside a
// file's run they are read and published now; a run publishes them as it
// commits, so no thread reads a class half-run.
fn record_change(mrb: &Mrb, _receiver: Value, module: Value) -> Value {
    let bookkeeping = bookkeeping(mrb);
    if let Some(row) = row_by_module(mrb, module) {
        let takers = bookkeeping.takers.borrow()[row].clone();
        bookkeeping.changed.borrow_mut().extend(takers);
    }
    if !bookkeeping.changed.borrow().is_empty() && !executor::is_file_running(mrb) {
        let mut draft = bookkeeping.snapshot.borrow_mut();
        record_changed_methods(mrb, &mut draft);
        snapshot::publish(Arc::clone(&draft));
    }
    qnil().as_value()
}

// Notes that the class of the file at `path` takes in `module`.
fn record_taker(mrb: &Mrb, module: Value, path: &str) {
    let bookkeeping = bookkeeping(mrb);
    let row = row_by_module(mrb, module).unwrap_or_else(|| {
        let mut takers = bookkeeping.takers.borrow_mut();
        takers.push(BTreeSet::new());
        let row = takers.len() - 1;
        bookkeeping
            .taken_in
            .set(mrb, module, (row as i64).into_value(mrb))
            .ok();
        row
    });
    bookkeeping.takers.borrow_mut()[row].insert(path.to_owned());
}

// The row of `takers` for `module`, if a class whose file has run takes it in.
fn row_by_module(mrb: &Mrb, module: Value) -> Option<usize> {
    let row = bookkeeping(mrb).taken_in.get(mrb, module).ok()?;
    usize::try_from(i64::try_convert(row, mrb).ok()?).ok()
}
