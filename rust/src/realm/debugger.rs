//! Stopping the realm's Ruby where a debugger asks: mruby's code fetch hook
//! runs before each instruction, and the first instruction of a line is
//! where a breakpoint or a step stops, as GDScript stops at each line.

use std::sync::Arc;

use beni::debug::{DebugInspector, TraceArg};

use super::{Location, Origin, Outermost, Realm, bookkeeping, leave_out_extension_frames};

/// The debugger a realm stops for. Its breakpoints and steps follow Godot's
/// script debugger: a step counts the lines left to pass, and its depth the
/// calls it has gone into since it began.
pub trait Debugger: Send {
    /// Whether `line` of `file` holds a breakpoint.
    fn is_breakpoint(&self, line: u32, file: &str) -> bool;
    /// The lines a step has left to pass, none while no step is under way.
    fn lines_left(&self) -> i32;
    fn set_lines_left(&self, lines: i32);
    /// How many calls deeper than its start a step is, negative when it
    /// stops in any of them.
    fn depth(&self) -> i32;
    fn set_depth(&self, depth: i32);
    /// Holds the Ruby at `stack`, nearest frame first, until the debugger
    /// lets it go on.
    fn stop(&self, stack: &[Location]);
    /// Lets the debugger take in what it was sent, as each line does.
    fn poll(&self);
}

impl Realm {
    /// Stops the Ruby this realm runs where `debugger` asks, from now on.
    pub fn attach_debugger(&mut self, debugger: impl Debugger + 'static) {
        let outermost = Arc::clone(&bookkeeping(&self.mrb).outermost);
        let mut lines = Lines::default();
        let mut depth = 0;
        self.mrb.set_code_fetch_hook(move |arg: TraceArg<'_>| {
            let Some(line) = arg.lineno() else { return };
            let inspector = arg.debug_inspector();
            let Some(running) = inspector.frame_depth(0) else {
                return;
            };
            if !lines.start(running, line.get()) {
                return;
            }
            let entered = running as i32 - depth;
            depth = running as i32;
            let file = arg.path().unwrap_or_default();
            if is_stopping(&debugger, entered, line.get() as u32, &file) {
                debugger.stop(&collect_stack(inspector, &outermost));
            }
            debugger.poll();
        });
    }
}

// Whether the line about to start in a frame `entered` calls deeper than the
// last one stops: a step reaching the end of its lines, or a breakpoint. As
// GDScript keeps a step's depth at each call and return, the calls between
// two lines are counted at the second.
fn is_stopping(debugger: &impl Debugger, entered: i32, line: u32, file: &str) -> bool {
    let mut stops = false;
    if debugger.lines_left() > 0 {
        if debugger.depth() >= 0 {
            debugger.set_depth(debugger.depth() + entered);
        }
        if debugger.depth() <= 0 {
            debugger.set_lines_left(debugger.lines_left() - 1);
        }
        stops = debugger.lines_left() <= 0;
    }
    stops || debugger.is_breakpoint(line, file)
}

// The frames of the running fiber that stand at a line, nearest first, as a
// backtrace keeps them. Under a call from Rust, the outermost frame is the
// top level of the last file run, which finished, so it is left out.
fn collect_stack(inspector: DebugInspector<'_>, outermost: &Outermost) -> Vec<Location> {
    let under_call = matches!(outermost.get(), Some(Origin::Call));
    let count = inspector.frame_depth(0).map_or(0, |depth| depth + 1);
    let mut stack: Vec<Location> = (0..count)
        .filter(|&index| !(under_call && index + 1 == count))
        .filter_map(|index| {
            let file = inspector.frame_path(index)?;
            let line = inspector.frame_lineno(index)?;
            Some(Location::new(file.into_owned(), line.get() as u32))
        })
        .collect();
    leave_out_extension_frames(&mut stack);
    stack
}

// The line each frame of the running fiber last started, outermost first, so
// a frame a call returned to goes on with its line rather than starting it
// again.
#[derive(Default)]
struct Lines(Vec<usize>);

impl Lines {
    // Whether `line` starts anew in the frame `depth` frames above the
    // outermost, which forgets the frames above it.
    fn start(&mut self, depth: usize, line: usize) -> bool {
        self.0.resize(depth + 1, 0);
        if self.0[depth] == line {
            return false;
        }
        self.0[depth] = line;
        true
    }
}
