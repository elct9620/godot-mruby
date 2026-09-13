//! Godot's log, written the way the `log` crate is: `log!` at any level,
//! `error!` and `warn!` for the common ones. `at:` places a record at a Ruby
//! file and line; without it, a record is placed where it was written.

use std::fmt;

use godot::global::{PrintLevel, PrintRecord, PrintSource, print_custom};

/// How serious a record is, as Godot's log tells them apart.
#[derive(Clone, Copy)]
pub enum Level {
    Error,
    Warn,
    /// A Ruby file that does not parse, the way Godot reports a GDScript one.
    ScriptError,
}

/// A line of a Ruby file.
pub struct Location {
    pub file: String,
    pub line: u32,
}

#[track_caller]
pub fn write<'a>(level: Level, at: impl Into<Option<&'a Location>>, message: fmt::Arguments) {
    let message = message.to_string();
    let source = match at.into() {
        Some(at) => PrintSource {
            function: "",
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
        $crate::log::write($level, None::<&$crate::log::Location>, format_args!($($arg)+))
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => { $crate::log!($crate::log::Level::Error, $($arg)+) };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => { $crate::log!($crate::log::Level::Warn, $($arg)+) };
}
