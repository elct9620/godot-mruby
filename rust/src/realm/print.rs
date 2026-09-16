//! `puts`, `print` and `p`, writing messages to the realm's log rather than
//! to the process's output: without an IO gem mruby has no output of its own,
//! and every realm prints.

use beni::format::Rest;
use beni::{Error, Module, Mrb, Value, method};

use super::bookkeeping;

pub(super) fn define(mrb: &Mrb) -> Result<(), Error> {
    let kernel = mrb.module_get(c"Kernel")?;
    kernel.define_private_method(mrb, c"puts", method!(puts, -1))?;
    kernel.define_private_method(mrb, c"print", method!(print, -1))?;
    kernel.define_private_method(mrb, c"p", method!(p, -1))
}

fn puts(mrb: &Mrb, _receiver: Value) -> Result<Value, Error> {
    let log = &bookkeeping(mrb).log;
    let args = mrb.get_args::<Rest>()?;
    if args.is_empty() {
        log.message("");
    }
    for arg in args {
        let text = arg.to_string(mrb);
        log.message(text.strip_suffix('\n').unwrap_or(&text));
    }
    Ok(Value::nil())
}

fn print(mrb: &Mrb, _receiver: Value) -> Result<Value, Error> {
    let args = mrb.get_args::<Rest>()?;
    let text: String = args.iter().map(|arg| arg.to_string(mrb)).collect();
    bookkeeping(mrb).log.raw(&text);
    Ok(Value::nil())
}

fn p(mrb: &Mrb, _receiver: Value) -> Result<Value, Error> {
    let log = &bookkeeping(mrb).log;
    let args = mrb.get_args::<Rest>()?;
    for arg in args {
        log.message(&arg.inspect(mrb));
    }
    Ok(match args {
        [] => Value::nil(),
        [arg] => *arg,
        args => mrb.ary_new_from_values(args).as_value(),
    })
}
