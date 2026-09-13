use std::collections::HashSet;
use std::ffi::CString;
use std::sync::Mutex;

use beni::format::Rest;
use beni::{Ccontext, Error, Module, Mrb, ParseMessage, Value, method};
use godot::global::{PrintLevel, PrintRecord, PrintSource, print_custom, printraw};
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
/// What mruby said about the run is returned rather than printed, so it is
/// reported outside the interpreter.
pub fn run_once(path: &str, source: impl FnOnce() -> String) -> Vec<Diagnostic> {
    let mut game = GAME.lock().unwrap();
    let interpreter = match game.as_mut() {
        Some(interpreter) => interpreter,
        None => match Interpreter::open() {
            Ok(interpreter) => game.insert(interpreter),
            Err(message) => return vec![Diagnostic::error(message)],
        },
    };
    if !interpreter.ran.insert(path.to_owned()) {
        return Vec::new();
    }
    interpreter.run(path, &source())
}

/// One thing mruby said about a file, reported through Godot's log.
pub struct Diagnostic {
    level: PrintLevel,
    message: String,
    // The file's path and the line mruby names; a report without one is
    // placed where it is reported from.
    at: Option<(String, u32)>,
}

impl Diagnostic {
    fn warning(path: &str, warning: &ParseMessage) -> Self {
        Self {
            level: PrintLevel::Warn,
            message: warning.message().to_owned(),
            at: Some((path.to_owned(), warning.line().into())),
        }
    }

    fn error(message: String) -> Self {
        Self {
            level: PrintLevel::Error,
            message,
            at: None,
        }
    }

    #[track_caller]
    pub fn report(&self) {
        let source = match &self.at {
            Some((file, line)) => PrintSource {
                function: "",
                file,
                line: *line,
            },
            None => PrintSource::caller(),
        };
        print_custom(PrintRecord {
            level: self.level,
            message: &self.message,
            rationale: None,
            source: Some(source),
            editor_notify: false,
        });
    }
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

    fn run(&self, path: &str, source: &str) -> Vec<Diagnostic> {
        let filename = match CString::new(path) {
            Ok(filename) => filename,
            Err(error) => return vec![Diagnostic::error(format!("{path}: {error}"))],
        };
        let Some(context) = Ccontext::new(&self.mrb, &filename) else {
            let message = format!("{path}: mruby could not make a compile context");
            return vec![Diagnostic::error(message)];
        };
        let outcome = context.load_nstring(source.as_bytes());
        let mut diagnostics: Vec<_> = context
            .warnings()
            .iter()
            .map(|warning| Diagnostic::warning(path, warning))
            .collect();
        if let Err(error) = outcome {
            let message = format!("{path}: {}", self.describe(&error));
            diagnostics.push(Diagnostic::error(message));
        }
        diagnostics
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
