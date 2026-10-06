//! `puts`, `print` and `p`, writing messages to the realm's log rather than
//! to the process's output: without an IO gem mruby has no output of its own,
//! and every realm prints.

use beni::{Error, Module, Mrb, ReprValue, Value, method, value::qnil};

use super::bookkeeping;

pub(super) fn define(mrb: &Mrb) -> Result<(), Error> {
    let kernel = mrb.module_get(c"Kernel")?;
    kernel.define_private_method(mrb, c"puts", method!(puts, -1))?;
    kernel.define_private_method(mrb, c"print", method!(print, -1))?;
    kernel.define_private_method(mrb, c"p", method!(p, -1))
}

fn puts(mrb: &Mrb, _receiver: Value, args: &[Value]) -> Value {
    let log = &bookkeeping(mrb).log;
    if args.is_empty() {
        log.print_line("");
    }
    for arg in args {
        let text = arg.to_string(mrb);
        log.print_line(text.strip_suffix('\n').unwrap_or(&text));
    }
    qnil().as_value()
}

fn print(mrb: &Mrb, _receiver: Value, args: &[Value]) -> Value {
    let text: String = args.iter().map(|arg| arg.to_string(mrb)).collect();
    bookkeeping(mrb).log.print(&text);
    qnil().as_value()
}

fn p(mrb: &Mrb, _receiver: Value, args: &[Value]) -> Value {
    let log = &bookkeeping(mrb).log;
    for arg in args {
        log.print_line(&arg.inspect(mrb));
    }
    match args {
        [] => qnil().as_value(),
        [arg] => *arg,
        args => mrb.ary_new_from_values(args).as_value(),
    }
}
