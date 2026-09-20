//! Running a file in a realm: once, and all or nothing. What a file creates
//! stays when it finishes and goes when it raises; what a file it loads by
//! name creates is that file's own.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::Arc;

use super::{bookkeeping, compile, file_defining, ran};
use crate::snapshot::{Group, Member, Property, Signal};
use beni::{Error, FromValue, Module, Mrb, RClass, ReprValue, Value};

/// How far a file has run in a realm.
#[derive(Clone, Copy)]
enum Run {
    Running,
    Done,
    /// It raised; by name it runs again, by path it does not.
    Failed,
}

/// What a class declares of itself as its body runs, for the realm to
/// publish once the file has run. One name is declared once, so the kinds
/// that carry a name are held together; a heading carries none.
pub(super) enum Declaration {
    Signal(Signal),
    Property(Property),
    Group(Group),
}

impl Declaration {
    fn name(&self) -> &str {
        match self {
            Self::Signal(signal) => &signal.name,
            Self::Property(property) => &property.name,
            Self::Group(group) => &group.name,
        }
    }

    // What the declaration says the name is, as a refusal spells it out.
    fn written(&self) -> String {
        match self {
            Self::Signal(signal) => format!("a signal of ({})", signal.parameters.join(", ")),
            Self::Property(property) => format!("a property of {}", property.default_value()),
            Self::Group(group) => format!("a heading named {}", group.name),
        }
    }
}

impl PartialEq for Declaration {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Signal(one), Self::Signal(other)) => one == other,
            (Self::Property(one), Self::Property(other)) => one == other,
            (Self::Group(one), Self::Group(other)) => one == other,
            _ => false,
        }
    }
}

/// A file running now, the constants it has created so far, each as the
/// names of its namespace and its own, and what its class has declared.
struct Frame {
    path: String,
    created: Vec<(Vec<String>, String)>,
    declared: Vec<Declaration>,
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

/// Whether the file at `path` is running now.
pub(super) fn running(mrb: &Mrb, path: &str) -> bool {
    matches!(runs(mrb).files.borrow().get(path), Some(Run::Running))
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

/// Records what `class`, whose file is running now, declares of itself. A
/// name is declared once: declaring it again as it stands is nothing new,
/// declaring it differently is refused where it is written, and a name an
/// ancestor declared is the ancestor's, refused in GDScript's own words so
/// that the same mistake reads the same in both languages.
pub(super) fn declare(mrb: &Mrb, class: RClass, declared: Declaration) -> Result<(), Error> {
    if let Some(ancestor) = ancestor_declaring(mrb, class, declared.name()) {
        return Err(raising(
            mrb,
            c"ArgumentError",
            &format!(
                "The member \"{}\" already exists in parent class {ancestor}.",
                declared.name()
            ),
        ));
    }
    let mut frames = runs(mrb).frames.borrow_mut();
    let Some(frame) = frames.last_mut() else {
        return Ok(());
    };
    let Some(standing) = frame
        .declared
        .iter()
        .find(|standing| standing.name() == declared.name())
    else {
        frame.declared.push(declared);
        return Ok(());
    };
    if standing == &declared {
        return Ok(());
    }
    let refusal = format!(
        "{} is already declared as {}, so it cannot be declared as {}",
        declared.name(),
        standing.written(),
        declared.written()
    );
    drop(frames);
    Err(raising(mrb, c"ArgumentError", &refusal))
}

// The ancestor of `class` that declared `name`, if one did. Each Ruby
// superclass names a file, and what that file declared is published once it
// has run; the engine's own members are asked of the engine instead, so the
// walk stops at the engine class.
fn ancestor_declaring(mrb: &Mrb, class: RClass, name: &str) -> Option<String> {
    let snapshot = Arc::clone(&bookkeeping(mrb).snapshot.borrow());
    let mut current = class.as_value();
    loop {
        current = current.funcall(mrb, c"superclass", &[]).ok()?;
        let path = RClass::from_value(current)?.path(mrb)?;
        if path.starts_with("Godot::") {
            return None;
        }
        let names: Vec<String> = path.split("::").map(str::to_owned).collect();
        let declared = file_defining(mrb, &names).is_some_and(|file| {
            snapshot.signals(&file).iter().any(|it| it.name == name)
                || snapshot.properties(&file).any(|it| it.name == name)
        });
        if declared {
            return Some(path);
        }
    }
}

/// Records the heading `group` written by the file running now, which names
/// no member of the class, so it is written as often as the class writes
/// one. A heading written while no file runs belongs to no class.
pub(super) fn heading(mrb: &Mrb, group: Group) {
    if let Some(frame) = runs(mrb).frames.borrow_mut().last_mut() {
        frame.declared.push(Declaration::Group(group));
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
        declared: Vec::new(),
    });
    let outcome = prepare().and_then(|prepared| {
        let source = source_of(mrb, path)?;
        let name = CString::new(path).map_err(|error| refused(mrb, &error.to_string()))?;
        compile(mrb, &name, &source)?;
        settle(prepared)
    });
    let frame = runs.frames.borrow_mut().pop();
    let run = match (&outcome, frame) {
        (Ok(()), frame) => {
            frame.into_iter().for_each(|frame| {
                let (signals, members) = split(frame.declared);
                ran(mrb, &frame.path, signals, members);
            });
            Run::Done
        }
        (Err(_), frame) => {
            frame.iter().for_each(|frame| take_away(mrb, frame));
            Run::Failed
        }
    };
    runs.files.borrow_mut().insert(path.to_owned(), run);
    outcome
}

// What a frame declared, as the two kinds a class publishes: its signals,
// and what the editor shows, which keeps the order it was declared in.
fn split(declared: Vec<Declaration>) -> (Vec<Signal>, Vec<Member>) {
    let mut signals = Vec::new();
    let mut members = Vec::new();
    for one in declared {
        match one {
            Declaration::Signal(signal) => signals.push(signal),
            Declaration::Property(property) => members.push(Member::Property(property)),
            Declaration::Group(group) => members.push(Member::Group(group)),
        }
    }
    (signals, members)
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
    raising(mrb, c"RuntimeError", message)
}

fn raising(mrb: &Mrb, class: &CStr, message: &str) -> Error {
    match mrb.exc_get(class) {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}
