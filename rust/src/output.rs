use beni::format::Rest;
use beni::{Error, Gem, Module, Mrb, Value, method};
use godot::global::printraw;
use godot::prelude::*;

/// `puts`, `print` and `p`, writing to Godot's output rather than the
/// process's: mruby core has no output of its own. Every realm installs it.
pub struct Output;

impl Gem for Output {
    fn init(mrb: &Mrb) -> Result<(), Error> {
        let kernel = mrb.module_get(c"Kernel")?;
        kernel.define_private_method(mrb, c"puts", method!(puts, -1))?;
        kernel.define_private_method(mrb, c"print", method!(print, -1))?;
        kernel.define_private_method(mrb, c"p", method!(p, -1))?;
        Ok(())
    }
}

fn puts(mrb: &Mrb, _receiver: Value) -> Value {
    let args = mrb.get_args::<Rest>();
    if args.is_empty() {
        godot_print!("");
    }
    for arg in args {
        let text = arg.to_string(mrb);
        godot_print!("{}", text.strip_suffix('\n').unwrap_or(&text));
    }
    Value::nil()
}

fn print(mrb: &Mrb, _receiver: Value) -> Value {
    let args = mrb.get_args::<Rest>();
    let text: String = args.iter().map(|arg| arg.to_string(mrb)).collect();
    printraw(&[text.to_variant()]);
    Value::nil()
}

fn p(mrb: &Mrb, _receiver: Value) -> Value {
    let args = mrb.get_args::<Rest>();
    for arg in args {
        godot_print!("{}", arg.inspect(mrb));
    }
    match args {
        [] => Value::nil(),
        [arg] => *arg,
        args => mrb.ary_new_from_values(args).as_value(),
    }
}
