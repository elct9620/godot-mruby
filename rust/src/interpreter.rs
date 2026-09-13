use std::collections::HashSet;
use std::ffi::CString;
use std::sync::Mutex;

use beni::format::Rest;
use beni::{Ccontext, Error, Module, Mrb, Value, method};
use godot::global::printraw;
use godot::prelude::*;

/// The interpreter every game file runs in: an `mrb_state` and the paths of
/// the files that have run in it. It opens at the first entry and one thread
/// at a time is inside it.
struct Interpreter {
    mrb: Mrb,
    ran: HashSet<String>,
}

static GAME: Mutex<Option<Interpreter>> = Mutex::new(None);

/// Runs the file at `path` in the game's interpreter unless it has run there
/// already, whether it succeeded or not; `source` is only read when it runs.
/// The error is the message to report, returned rather than printed so it is
/// reported outside the interpreter.
pub fn run_once(path: &str, source: impl FnOnce() -> String) -> Result<(), String> {
    let mut game = GAME.lock().unwrap();
    let interpreter = match game.as_mut() {
        Some(interpreter) => interpreter,
        None => game.insert(Interpreter::open()?),
    };
    if !interpreter.ran.insert(path.to_owned()) {
        return Ok(());
    }
    interpreter.run(path, &source())
}

pub fn close() {
    GAME.lock().unwrap().take();
}

impl Interpreter {
    fn open() -> Result<Self, String> {
        let mrb = Mrb::open().map_err(|error| format!("mruby did not open: {error}"))?;
        define_output(&mrb).map_err(|error| error.message(&mrb))?;
        Ok(Self {
            mrb,
            ran: HashSet::new(),
        })
    }

    fn run(&self, path: &str, source: &str) -> Result<(), String> {
        let filename = CString::new(path).map_err(|error| format!("{path}: {error}"))?;
        let context = Ccontext::new(&self.mrb, &filename)
            .ok_or_else(|| format!("{path}: mruby could not make a compile context"))?;
        context
            .load_nstring(source.as_bytes())
            .map(drop)
            .map_err(|error| format!("{path}: {}", self.describe(&error)))
    }

    // An exception renders only through the interpreter it was raised in; a
    // syntax error carries its own line and column.
    fn describe(&self, error: &Error) -> String {
        match error {
            Error::Exception(_) => error.message(&self.mrb),
            _ => error.to_string(),
        }
    }
}

/// mruby core has no output of its own, and Ruby prints to Godot's output
/// rather than to the process's.
fn define_output(mrb: &Mrb) -> Result<(), Error> {
    let kernel = mrb.module_get(c"Kernel")?;
    kernel.define_private_method(mrb, c"puts", method!(puts, -1))?;
    kernel.define_private_method(mrb, c"print", method!(print, -1))?;
    kernel.define_private_method(mrb, c"p", method!(p, -1))?;
    Ok(())
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
