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

// The test framework ships inside the extension but only reaches an
// interpreter the test runner prepares; each file's path names its frames in
// backtraces, and the framework tells its own frames by that directory.
const FRAMEWORK: [(&str, &str); 2] = [
    ("godot_mruby/minitest.rb", include_str!("minitest.rb")),
    ("godot_mruby/mock.rb", include_str!("mock.rb")),
];
// Where the runner compiles its lookup of the framework, as mruby names it
// in anything it says about that lookup.
const RUNNER_PATH: &str = "godot_mruby/runner";

/// Runs the file at `path` in the game's interpreter unless it has run there
/// already, whether it succeeded or not; `source` is only read when it runs.
/// What mruby said about the run is returned rather than printed, so it is
/// reported outside the interpreter.
pub fn run_once(path: &str, source: impl FnOnce() -> String) -> Vec<Diagnostic> {
    with_game(|game| game.run_once(path, source)).unwrap_or_else(|failed| vec![failed])
}

/// Runs the test framework and then each test file in the game's
/// interpreter, and answers what mruby said about them and whether every one
/// loaded: a test file that did not load fails the run as a failed test
/// would. `source` reads a file at its path.
pub fn load_tests(paths: &[String], source: impl Fn(&str) -> String) -> (Vec<Diagnostic>, bool) {
    let diagnostics = with_game(|game| {
        let mut diagnostics = Vec::new();
        for (path, source) in FRAMEWORK {
            diagnostics.extend(game.run_once(path, || source.to_owned()));
        }
        for path in paths {
            diagnostics.extend(game.run_once(path, || source(path)));
        }
        diagnostics
    })
    .unwrap_or_else(|failed| vec![failed]);
    let loaded = !diagnostics.iter().any(Diagnostic::is_error);
    (diagnostics, loaded)
}

/// What narrows a test run, as the runner's command line gives it.
pub struct TestOptions {
    /// The test methods to run alone, named by name or as `Class#name`.
    pub include: Option<String>,
    /// The test methods to leave out, named by name or as `Class#name`.
    pub exclude: Option<String>,
    /// The seed that orders the tests; a random one when there is none.
    pub seed: Option<i64>,
}

/// Runs the tests the loaded test files defined that `options` leaves, and
/// answers what went wrong, each where it went wrong; nothing means every
/// test passed.
pub fn run_tests(options: &TestOptions) -> Vec<Diagnostic> {
    with_game(|game| game.run_minitest(options)).unwrap_or_else(|failed| vec![failed])
}

fn with_game<T>(enter: impl FnOnce(&mut Interpreter) -> T) -> Result<T, Diagnostic> {
    let mut game = GAME.lock().unwrap();
    let interpreter = match game.as_mut() {
        Some(interpreter) => interpreter,
        None => game.insert(Interpreter::open().map_err(Diagnostic::error)?),
    };
    Ok(enter(interpreter))
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
    fn compiled(level: PrintLevel, path: &str, said: &ParseMessage) -> Self {
        Self {
            level,
            message: said.message().to_owned(),
            at: Some((path.to_owned(), said.line().into())),
        }
    }

    fn error(message: String) -> Self {
        Self {
            level: PrintLevel::Error,
            message,
            at: None,
        }
    }

    fn is_error(&self) -> bool {
        matches!(self.level, PrintLevel::Error | PrintLevel::ScriptError)
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

    fn run_once(&mut self, path: &str, source: impl FnOnce() -> String) -> Vec<Diagnostic> {
        if !self.ran.insert(path.to_owned()) {
            return Vec::new();
        }
        self.run(path, &source())
    }

    // Every value mruby hands back here is read into Rust before the arena
    // scope ends.
    fn run_minitest(&self, options: &TestOptions) -> Vec<Diagnostic> {
        let _scope = self.mrb.arena_scope();
        let context = match self.context(RUNNER_PATH) {
            Ok(context) => context,
            Err(failed) => return vec![failed],
        };
        let problems = self
            .options_hash(options)
            .and_then(|options| Ok((context.load_nstring(b"Minitest")?, options)))
            .and_then(|(minitest, options)| minitest.funcall(&self.mrb, c"run", &[options]))
            .and_then(|problems| problems.ensure_array(&self.mrb));
        match problems {
            Ok(problems) => (0..problems.len())
                .map(|index| self.problem(problems.entry(index as isize)))
                .collect(),
            Err(error) => vec![self.diagnose(RUNNER_PATH, &error)],
        }
    }

    // The options Minitest.run takes, keyed by symbols as minitest keys them.
    fn options_hash(&self, options: &TestOptions) -> Result<Value, Error> {
        let hash = self.mrb.hash_new();
        for (key, value) in [("include", &options.include), ("exclude", &options.exclude)] {
            if let Some(value) = value {
                let key = self.mrb.intern(key.as_bytes()).as_value();
                hash.set(
                    &self.mrb,
                    key,
                    self.mrb.str_new(value.as_bytes()).as_value(),
                )?;
            }
        }
        if let Some(seed) = options.seed {
            let key = self.mrb.intern(b"seed").as_value();
            hash.set(&self.mrb, key, Value::from_int(&self.mrb, seed))?;
        }
        Ok(hash.as_value())
    }

    // One problem Minitest.run found: its message, then the file and line it
    // happened at, or nil for both.
    fn problem(&self, problem: Value) -> Diagnostic {
        let Ok(fields) = problem.ensure_array(&self.mrb) else {
            return Diagnostic::error(problem.inspect(&self.mrb));
        };
        let file = fields.entry(1);
        let line = fields.entry(2).to_string(&self.mrb);
        Diagnostic {
            level: PrintLevel::Error,
            message: fields.entry(0).to_string(&self.mrb),
            at: (!file.is_nil()).then(|| (file.to_string(&self.mrb), line.parse().unwrap_or(0))),
        }
    }

    fn run(&self, path: &str, source: &str) -> Vec<Diagnostic> {
        let context = match self.context(path) {
            Ok(context) => context,
            Err(failed) => return vec![failed],
        };
        let outcome = context.load_nstring(source.as_bytes());
        let mut diagnostics: Vec<_> = context
            .warnings()
            .iter()
            .map(|warning| Diagnostic::compiled(PrintLevel::Warn, path, warning))
            .collect();
        if let Err(error) = outcome {
            diagnostics.push(self.diagnose(path, &error));
        }
        diagnostics
    }

    // What `path` compiles in: mruby stamps it on everything compiled there,
    // so warnings, errors and backtraces name it.
    fn context(&self, path: &str) -> Result<Ccontext<'_>, Diagnostic> {
        let filename =
            CString::new(path).map_err(|error| Diagnostic::error(format!("{path}: {error}")))?;
        Ccontext::new(&self.mrb, &filename).ok_or_else(|| {
            Diagnostic::error(format!("{path}: mruby could not make a compile context"))
        })
    }

    // A syntax error names its own line, the way Godot reports a script that
    // does not parse; an exception renders only through the interpreter it
    // was raised in.
    fn diagnose(&self, path: &str, error: &Error) -> Diagnostic {
        match error {
            Error::Syntax(parse) => Diagnostic::compiled(PrintLevel::ScriptError, path, parse),
            Error::Exception(_) => {
                Diagnostic::error(format!("{path}: {}", error.message(&self.mrb)))
            }
            _ => Diagnostic::error(format!("{path}: {error}")),
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
