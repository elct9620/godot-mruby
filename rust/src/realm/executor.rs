//! Running a file in a realm: once, and all or nothing. What a file creates
//! stays when it finishes and goes when it raises; what a file it loads by
//! name creates is that file's own.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;

use super::{bookkeeping, compile};
use beni::{Error, Mrb, ReprValue, Value};

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
/// `prepare` is the run's first step, and what it answers `settle` is given
/// as the last, once the file has run.
pub(super) fn run<T>(
    mrb: &Mrb,
    path: &str,
    prepare: impl FnOnce() -> Result<T, Error>,
    settle: impl FnOnce(T) -> Result<(), Error>,
) -> Result<(), Error> {
    if runs(mrb).files.borrow().contains_key(path) {
        return Ok(());
    }
    execute(mrb, path, prepare, settle)
}

/// Runs the file at `path` for a name it defines, unless it has run cleanly
/// or is running now. A file that raised runs again, as a failed `require`
/// does in Ruby: it took away what it created, so it starts over.
pub(super) fn run_by_name<T>(
    mrb: &Mrb,
    path: &str,
    prepare: impl FnOnce() -> Result<T, Error>,
    settle: impl FnOnce(T) -> Result<(), Error>,
) -> Result<(), Error> {
    match runs(mrb).files.borrow().get(path) {
        Some(Run::Done | Run::Running) => return Ok(()),
        Some(Run::Failed) | None => {}
    }
    execute(mrb, path, prepare, settle)
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

fn runs(mrb: &Mrb) -> &Runs {
    &bookkeeping(mrb).runs
}

fn execute<T>(
    mrb: &Mrb,
    path: &str,
    prepare: impl FnOnce() -> Result<T, Error>,
    settle: impl FnOnce(T) -> Result<(), Error>,
) -> Result<(), Error> {
    let runs = runs(mrb);
    runs.files
        .borrow_mut()
        .insert(path.to_owned(), Run::Running);
    runs.frames.borrow_mut().push(Frame {
        path: path.to_owned(),
        created: Vec::new(),
    });
    let outcome = prepare().and_then(|prepared| {
        let source = source_of(mrb, path)?;
        let name = CString::new(path).map_err(|error| refused(mrb, &error.to_string()))?;
        compile(mrb, &name, &source)?;
        settle(prepared)
    });
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

// The source the realm's files give for the file at `path`.
fn source_of(mrb: &Mrb, path: &str) -> Result<String, Error> {
    bookkeeping(mrb)
        .files
        .source(path)
        .map_err(|why| refused(mrb, &why))
}

// Removes what `frame` created, the last first, so a constant inside a class
// goes before the class.
fn take_away(mrb: &Mrb, frame: &Frame) {
    for (scope, name) in frame.created.iter().rev() {
        if let Some(scope) = defined_scope(mrb, scope) {
            scope.const_remove(mrb, name.as_str()).ok();
        }
    }
}

// The module the names in `scope` spell exactly, if each is still defined;
// asking for a missing one would load it by name.
fn defined_scope(mrb: &Mrb, scope: &[String]) -> Option<Value> {
    scope
        .iter()
        .try_fold(mrb.object_class().as_value(), |outer, name| {
            let name = mrb.intern(name.as_bytes()).ok()?;
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
