//! Running a file in a realm: once, and all or nothing. What a file creates
//! stays when it finishes and goes when it raises; what a file it loads by
//! name creates is that file's own.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;

use beni::{Ccontext, Error, Mrb, Value};
use godot::classes::{ResourceLoader, Script};
use godot::obj::Singleton;

use super::bookkeeping;
use crate::log::Location;
use crate::warn;

/// How far a file has run in a realm.
#[derive(Clone, Copy)]
enum Run {
    Running,
    Done,
    /// It raised; by name it runs again, by path it does not.
    Failed,
}

/// A file running now, and the constants it has created so far, each as the
/// names of its namespace and its own.
struct Frame {
    path: String,
    created: Vec<(Vec<String>, String)>,
}

/// How far each file has run in a realm, and the files running now.
#[derive(Default)]
pub(super) struct Runs {
    files: RefCell<HashMap<String, Run>>,
    frames: RefCell<Vec<Frame>>,
}

/// Runs the file at `path` by path: once, whether it succeeded or not.
/// `prepare` is the run's first step.
pub(super) fn run(
    mrb: &Mrb,
    path: &str,
    prepare: impl FnOnce() -> Result<(), Error>,
) -> Result<(), Error> {
    if runs(mrb).files.borrow().contains_key(path) {
        return Ok(());
    }
    execute(mrb, path, prepare)
}

/// Runs the file at `path` for a name it defines, unless it has run cleanly
/// or is running now. A file that raised runs again, as a failed `require`
/// does in Ruby: it took away what it created, so it starts over.
pub(super) fn run_by_name(
    mrb: &Mrb,
    path: &str,
    prepare: impl FnOnce() -> Result<(), Error>,
) -> Result<(), Error> {
    match runs(mrb).files.borrow().get(path) {
        Some(Run::Done | Run::Running) => return Ok(()),
        Some(Run::Failed) | None => {}
    }
    execute(mrb, path, prepare)
}

/// The files that led back to the one at `path` while it is still running,
/// ending with it; nothing when it is not running.
pub(super) fn cycle(mrb: &Mrb, path: &str) -> Option<Vec<String>> {
    let runs = runs(mrb);
    if !matches!(runs.files.borrow().get(path), Some(Run::Running)) {
        return None;
    }
    let frames = runs.frames.borrow();
    let start = frames
        .iter()
        .position(|frame| frame.path == path)
        .unwrap_or(0);
    Some(
        frames[start..]
            .iter()
            .map(|frame| frame.path.clone())
            .chain([path.to_owned()])
            .collect(),
    )
}

/// Records that the file running now created `name` in the namespace the
/// names in `scope` spell.
pub(super) fn record(mrb: &Mrb, scope: Vec<String>, name: String) {
    if let Some(frame) = runs(mrb).frames.borrow_mut().last_mut() {
        frame.created.push((scope, name));
    }
}

/// Runs `source` as the file at `path`: mruby stamps the path on everything
/// compiled from it, so warnings, errors and backtraces name it. What the
/// compiler warns about is written to Godot's log at its line.
pub(super) fn load(mrb: &Mrb, path: &str, source: &str) -> Result<(), Error> {
    let filename = CString::new(path).map_err(|error| refused(mrb, &error.to_string()))?;
    let context = Ccontext::new(mrb, &filename)
        .ok_or_else(|| refused(mrb, "mruby could not make a compile context"))?;
    let outcome = context.load_nstring(source.as_bytes());
    for warning in context.warnings() {
        let at = Location {
            file: path.to_owned(),
            line: warning.line().into(),
        };
        warn!(at: &at, "{}", warning.message());
    }
    outcome.map(|_| ())
}

fn runs(mrb: &Mrb) -> &Runs {
    &bookkeeping(mrb).runs
}

fn execute(
    mrb: &Mrb,
    path: &str,
    prepare: impl FnOnce() -> Result<(), Error>,
) -> Result<(), Error> {
    let runs = runs(mrb);
    runs.files
        .borrow_mut()
        .insert(path.to_owned(), Run::Running);
    runs.frames.borrow_mut().push(Frame {
        path: path.to_owned(),
        created: Vec::new(),
    });
    let outcome = prepare()
        .and_then(|()| source_of(mrb, path))
        .and_then(|source| load(mrb, path, &source));
    let frame = runs.frames.borrow_mut().pop();
    let run = match (&outcome, frame) {
        (Ok(()), _) => Run::Done,
        (Err(_), frame) => {
            frame.iter().for_each(|frame| take_away(mrb, frame));
            Run::Failed
        }
    };
    runs.files.borrow_mut().insert(path.to_owned(), run);
    outcome
}

// The source Godot holds for the file at `path`.
fn source_of(mrb: &Mrb, path: &str) -> Result<String, Error> {
    ResourceLoader::singleton()
        .load_ex(path)
        .type_hint("Script")
        .done()
        .and_then(|resource| resource.try_cast::<Script>().ok())
        .map(|script| script.get_source_code().to_string())
        .ok_or_else(|| refused(mrb, "Godot did not load it as a script"))
}

// Removes what `frame` created, the last first, so a constant inside a class
// goes before the class.
fn take_away(mrb: &Mrb, frame: &Frame) {
    for (scope, name) in frame.created.iter().rev() {
        if let (Some(scope), Ok(name)) = (defined_scope(mrb, scope), mrb.intern(name.as_bytes())) {
            scope.const_remove(mrb, name.to_sym()).ok();
        }
    }
}

// The module the names in `scope` spell exactly, if each is still defined;
// asking for a missing one would load it by name.
fn defined_scope(mrb: &Mrb, scope: &[String]) -> Option<Value> {
    scope
        .iter()
        .try_fold(mrb.object_class().to_value(mrb), |outer, name| {
            let name = mrb.intern(name.as_bytes()).ok()?.to_sym();
            if outer.const_defined_at(mrb, name) {
                outer.const_get(mrb, name).ok()
            } else {
                None
            }
        })
}

fn refused(mrb: &Mrb, message: &str) -> Error {
    match mrb.exc_get(c"RuntimeError") {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}
