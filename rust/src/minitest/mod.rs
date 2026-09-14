//! The test framework: minitest's design and spelling in Ruby, installed
//! into a realm only by the test runner, so the shipped game never has it.

use beni::format::Rest;
use beni::{Error, Gem, IntoValue, Module, Mrb, Value, method};

use crate::error;
use crate::log::Location;
use crate::realm;

// Each file's path names its frames in backtraces: the framework tells its
// own frames by their directory, and the extension's by the one above it.
const FILES: [(&str, &str); 3] = [
    (
        "godot_mruby/minitest/minitest.rb",
        include_str!("minitest.rb"),
    ),
    ("godot_mruby/minitest/mock.rb", include_str!("mock.rb")),
    (
        "godot_mruby/minitest/godot_plugin.rb",
        include_str!("godot_plugin.rb"),
    ),
];

pub struct Minitest;

impl Gem for Minitest {
    fn init(mrb: &Mrb) -> Result<(), Error> {
        for (path, source) in FILES {
            realm::load(mrb, path, source)?;
        }
        mrb.module_get(c"Minitest")?
            .class_get(mrb, c"LogReporter")?
            .define_private_method(mrb, c"error", method!(log_error, -1))
    }
}

// Minitest::LogReporter#error(message, file, line): a test that did not
// pass, written to Godot's log at its Ruby line when it has one.
fn log_error(mrb: &Mrb, _reporter: Value) -> Result<Value, Error> {
    let args = mrb.get_args::<Rest>()?;
    let [message, file, line] = args else {
        return Err(Error::argnum(mrb, args.len() as i64, 3, 3));
    };
    let at = (!file.is_nil()).then(|| Location {
        file: file.to_string(mrb),
        line: line
            .as_int(mrb)
            .ok()
            .and_then(|line| line.try_into().ok())
            .unwrap_or(0),
    });
    error!(at: at.as_ref(), "{}", message.to_string(mrb));
    Ok(Value::nil())
}

/// What narrows a run, as `Minitest.run` takes it.
pub struct Options {
    /// The test methods to run alone, named by name or as `Class#name`.
    pub include: Option<String>,
    /// The test methods to leave out, named by name or as `Class#name`.
    pub exclude: Option<String>,
    /// The seed that orders the tests; a random one when there is none.
    pub seed: Option<i64>,
}

impl IntoValue for Options {
    // Keyed by symbols, as minitest keys its options.
    fn into_value(self, mrb: &Mrb) -> Value {
        let hash = mrb.hash_new();
        let entries = [
            (
                "include",
                self.include
                    .map(|value| mrb.str_new(value.as_bytes()).as_value()),
            ),
            (
                "exclude",
                self.exclude
                    .map(|value| mrb.str_new(value.as_bytes()).as_value()),
            ),
            ("seed", self.seed.map(|seed| Value::from_int(mrb, seed))),
        ];
        for (key, value) in entries {
            if let Some(value) = value {
                let key = mrb.intern(key.as_bytes()).expect("a short name interns");
                hash.set(mrb, key.as_value(), value)
                    .expect("a fresh hash takes a symbol key");
            }
        }
        hash.as_value()
    }
}
