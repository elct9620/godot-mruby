//! Godot's log, written the way the `log` crate is: `log!` at any level,
//! `error!` and `warn!` for the common ones. `at:` places a record at a Ruby
//! file and line; without it, a record is placed where it was written.
//! `GodotLog` is the same log as a realm writes to it.

use std::ffi::CStr;
use std::fmt;

use godot::global::{PrintLevel, PrintRecord, PrintSource, godot_print, print_custom, printraw};
use godot::prelude::ToGodot;

use crate::compiler::Warning;
use crate::realm::{self, Level, Location};

/// Godot's log as a realm writes to it.
pub struct GodotLog;

impl realm::Log for GodotLog {
    fn message(&self, text: &str) {
        godot_print!("{text}");
    }

    fn raw(&self, text: &str) {
        printraw(&[text.to_variant()]);
    }

    #[track_caller]
    fn record(&self, level: Level, at: Option<&Location>, text: &str) {
        write(level, at, format_args!("{text}"));
    }
}

/// Writes what the compiler warns about in `file`, Ruby the extension ships
/// with, to Godot's log at its line.
pub fn compiler_warnings(file: &CStr) -> impl FnMut(Warning) {
    let file = file.to_string_lossy().into_owned();
    move |warning| {
        let at = Location {
            file: file.clone(),
            line: warning.line,
            function: String::new(),
        };
        crate::warn!(at: &at, "{}", warning.message);
    }
}

#[track_caller]
pub fn write<'a>(level: Level, at: impl Into<Option<&'a Location>>, message: fmt::Arguments) {
    let message = message.to_string();
    let source = match at.into() {
        Some(at) => PrintSource {
            function: &at.function,
            file: &at.file,
            line: at.line,
        },
        None => PrintSource::caller(),
    };
    print_custom(PrintRecord {
        level: match level {
            Level::Error => PrintLevel::Error,
            Level::Warn => PrintLevel::Warn,
            Level::ScriptError => PrintLevel::ScriptError,
        },
        message: &message,
        rationale: None,
        source: Some(source),
        editor_notify: false,
    });
}

#[macro_export]
macro_rules! log {
    ($level:expr, at: $at:expr, $($arg:tt)+) => {
        $crate::log::write($level, $at, format_args!($($arg)+))
    };
    ($level:expr, $($arg:tt)+) => {
        $crate::log::write($level, None::<&$crate::realm::Location>, format_args!($($arg)+))
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => { $crate::log!($crate::realm::Level::Error, $($arg)+) };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => { $crate::log!($crate::realm::Level::Warn, $($arg)+) };
}
