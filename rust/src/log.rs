//! Godot's log, written the way the `log` crate is: `log!` at any level,
//! `error!` and `warn!` for the common ones. `at:` places a record at a Ruby
//! file and line; without it, a record is placed where it was written.
//! `GodotLog` is the same log as a realm writes to it.

use std::cell::RefCell;
use std::ffi::CStr;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};

use godot::classes::Os;
use godot::global::{PrintLevel, PrintRecord, PrintSource, godot_print, print_custom, printraw};
use godot::obj::Singleton;
use godot::prelude::ToGodot;

use crate::compiler::Warning;
use crate::realm::{self, Level, Location};

/// Godot's log as a realm writes to it.
pub struct GodotLog;

impl realm::Log for GodotLog {
    fn print_line(&self, text: &str) {
        godot_print!("{text}");
    }

    fn print(&self, text: &str) {
        printraw(&[text.to_variant()]);
    }

    #[track_caller]
    fn record(&self, level: Level, at: Option<&Location>, text: &str) {
        write(level, at, format_args!("{text}"));
    }

    // Godot asks every language for its stack as it prints, so the language
    // answers the exception's backtrace while it is written.
    fn record_exception(&self, text: &str, backtrace: &[Location]) {
        write_with_backtrace(backtrace, || {
            write(
                Level::ScriptError,
                backtrace.first(),
                format_args!("{text}"),
            );
        });
    }
}

/// The thread calling, as the log names it: the main thread, or another by
/// the id `OS.get_thread_caller_id` gives it.
pub fn name_thread() -> String {
    if godot::sys::is_main_thread() {
        return "the main thread".to_owned();
    }
    format!("thread {}", Os::singleton().get_thread_caller_id())
}

thread_local! {
    // The backtrace of what this thread is reporting. It holds no Godot
    // value, since a thread's locals outlive the engine.
    static BACKTRACE: RefCell<Vec<Location>> = const { RefCell::new(Vec::new()) };
}

/// Runs `write` while `backtrace` is the one this thread is reporting, so
/// what Godot prints in it carries that backtrace.
pub fn write_with_backtrace<R>(backtrace: &[Location], write: impl FnOnce() -> R) -> R {
    let outer = BACKTRACE.replace(backtrace.to_vec());
    let answer = write();
    BACKTRACE.set(outer);
    answer
}

/// The backtrace of what this thread is reporting, which the language
/// answers as its stack, since Godot asks for it as it prints.
pub fn current_backtrace() -> Vec<Location> {
    BACKTRACE.with_borrow(Clone::clone)
}

/// Writes what the compiler warns about in `file`, Ruby the extension ships
/// with, to Godot's log at its line.
pub fn write_compiler_warnings(file: &CStr) -> impl FnMut(Warning) {
    let file = file.to_string_lossy().into_owned();
    move |warning| {
        let at = Location::new(file.clone(), warning.line);
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

/// Answers what `body` answers, or `fallback` when it panics, writing the
/// panic to Godot's log: a panic unwinding out of a callback Godot called
/// would abort the process. What a script instance shares across callbacks
/// is behind atomics and mutexes that recover from a panic, so `body` is
/// taken as unwind safe.
pub fn catch_panic<R>(callback: &str, fallback: R, body: impl FnOnce() -> R) -> R {
    panic::catch_unwind(AssertUnwindSafe(body)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("an unknown panic");
        crate::error!("{callback} on a Ruby script instance failed: {message}");
        fallback
    })
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
