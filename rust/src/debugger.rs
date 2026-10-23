//! Godot's script debugger as the game's realm stops for it: breakpoints and
//! steps are the engine's, and a stop is a debug break the language answers
//! the stack of, as GDScript breaks.

use std::cell::Cell;
use std::sync::{Mutex, PoisonError};

use godot::builtin::{GString, StringName};
use godot::classes::{EngineDebugger, ScriptLanguage};
use godot::obj::Singleton;

use crate::realm::{self, Location, Realm};
use crate::{language, log};

// Why a debug break stops, as GDScript words it for a breakpoint and a step.
const BREAKPOINT: &str = "Breakpoint";

thread_local! {
    // Whether this thread is stopped at a debug break.
    static STOPPED: Cell<bool> = const { Cell::new(false) };
}

/// `realm`, stopping its Ruby at Godot's breakpoints and steps when the game
/// runs under a debugger.
pub fn attach(mut realm: Realm) -> Realm {
    if EngineDebugger::singleton().is_active() {
        realm.attach_debugger(GodotDebugger::default());
    }
    realm
}

/// Why this thread's debug break stopped, empty while it is not stopped.
pub fn stop_reason() -> GString {
    match STOPPED.get() {
        true => GString::from(BREAKPOINT),
        false => GString::new(),
    }
}

#[derive(Default)]
struct GodotDebugger {
    // The last file asked about, with the name Godot keeps its breakpoints
    // under: lines follow each other in one file far more often than not.
    file: Mutex<(String, StringName)>,
}

impl realm::Debugger for GodotDebugger {
    fn is_breakpoint(&self, line: u32, file: &str) -> bool {
        let mut last = self.file.lock().unwrap_or_else(PoisonError::into_inner);
        if last.0 != file {
            *last = (file.to_owned(), StringName::from(file));
        }
        EngineDebugger::singleton().is_breakpoint(line as i32, &last.1)
    }

    fn lines_left(&self) -> i32 {
        EngineDebugger::singleton().get_lines_left()
    }

    fn set_lines_left(&self, lines: i32) {
        EngineDebugger::singleton().set_lines_left(lines);
    }

    fn depth(&self) -> i32 {
        EngineDebugger::singleton().get_depth()
    }

    fn set_depth(&self, depth: i32) {
        EngineDebugger::singleton().set_depth(depth);
    }

    // The language answers `stack` while Godot holds the break, as it
    // answers an exception's backtrace while one is written.
    fn stop(&self, stack: &[Location]) {
        let Some(language) = language::language() else {
            return;
        };
        log::report_with_backtrace(stack, || {
            STOPPED.set(true);
            EngineDebugger::singleton().script_debug(&language.upcast::<ScriptLanguage>());
            STOPPED.set(false);
        });
    }

    fn poll(&self) {
        EngineDebugger::singleton().line_poll();
    }
}
