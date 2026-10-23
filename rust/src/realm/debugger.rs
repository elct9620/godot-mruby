//! Stopping the realm's Ruby where a debugger asks: mruby's code fetch hook
//! runs before each instruction, and the first instruction of a line is
//! where a breakpoint or a step stops, as GDScript stops at each line. The
//! Ruby the extension ships is stepped over, as its frames are left out of a
//! backtrace.

use std::sync::Arc;

use beni::Mrb;
use beni::debug::{DebugInspector, TraceArg};

use super::{
    EXTENSION_DIRECTORY, Location, Origin, Outermost, Realm, bookkeeping,
    leave_out_extension_frames,
};

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
            let file = arg.path().unwrap_or_default();
            if file.starts_with(EXTENSION_DIRECTORY) {
                return;
            }
            lines.claim(running);
            let entered = lines.count_project_frames() - depth;
            depth += entered;
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
        .filter_map(|index| locate_frame(inspector, index))
        .collect();
    leave_out_extension_frames(&mut stack);
    stack
}

/// The frames of the running fiber in `mrb` that stand at a line, nearest
/// first, each in the method it runs, read off the frame stack without
/// running Ruby.
pub(super) fn read_frames(mrb: &Mrb) -> Vec<Location> {
    let mut frames = Vec::new();
    mrb.debug_inspector_open(|inspector| {
        let count = inspector.frame_depth(0).map_or(0, |depth| depth + 1);
        frames = (0..count)
            .filter_map(|index| {
                let mut frame = locate_frame(inspector, index)?;
                if let Some(id) = inspector.frame_method_id(index) {
                    frame.function = id.name(mrb).unwrap_or_default();
                }
                Some(frame)
            })
            .collect();
    })
    .ok();
    frames
}

// Where the frame `index` stands, in no method it names; nothing for a frame
// running Rust or C code, or code that carries no line numbers.
fn locate_frame(inspector: DebugInspector<'_>, index: usize) -> Option<Location> {
    let file = inspector.frame_path(index)?;
    let line = inspector.frame_lineno(index)?;
    Some(Location::new(file.into_owned(), line.get() as u32))
}

// The line each frame of the running fiber last started, outermost first, so
// a frame a call returned to goes on with its line rather than starting it
// again; and whether that line is the project's, so a step's depth counts
// the project's frames alone, as GDScript's counts its functions.
#[derive(Default)]
struct Lines(Vec<FrameLine>);

#[derive(Clone, Copy, Default)]
struct FrameLine {
    line: usize,
    is_project: bool,
}

impl Lines {
    // Whether `line` starts anew in the frame `depth` frames above the
    // outermost, which forgets the frames above it.
    fn start(&mut self, depth: usize, line: usize) -> bool {
        self.0.resize(depth + 1, FrameLine::default());
        if self.0[depth].line == line {
            return false;
        }
        self.0[depth] = FrameLine {
            line,
            is_project: false,
        };
        true
    }

    // Marks the line the frame `depth` started as the project's.
    fn claim(&mut self, depth: usize) {
        self.0[depth].is_project = true;
    }

    fn count_project_frames(&self) -> i32 {
        self.0.iter().filter(|frame| frame.is_project).count() as i32
    }
}
