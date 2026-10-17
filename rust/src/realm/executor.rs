//! Running a file in a realm: once, and all or nothing. What a file creates
//! stays when it finishes and goes when it raises; what a file it loads by
//! name creates is that file's own. A file run again keeps whatever it
//! changes, as Ruby's `load` does, since the objects it made live on.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::Arc;

use super::loader::{self, Scope};
use super::{
    bookkeeping, compile, file_by_constant, join_inner_path, key_by_path, methods, normalize,
};
use crate::snapshot::{self, Heading, Member, MethodCalls, Property, Rpc, Signal};
use beni::{Class as _, Error, FromValue, Module, Mrb, RClass};

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
    Heading(Heading),
    Rpc(Rpc),
}

impl Declaration {
    fn name(&self) -> &str {
        match self {
            Self::Signal(signal) => &signal.name,
            Self::Property(property) => &property.name,
            Self::Heading(heading) => heading.name(),
            Self::Rpc(rpc) => &rpc.method,
        }
    }

    // What the declaration says the name is, as a refusal spells it out.
    fn label(&self) -> String {
        match self {
            Self::Signal(signal) => {
                let names: Vec<&str> = signal
                    .parameters
                    .iter()
                    .map(|parameter| parameter.name.as_str())
                    .collect();
                format!("a signal of ({})", names.join(", "))
            }
            Self::Property(property) => format!("a property of {}", property.default_text()),
            Self::Heading(heading) => format!("a heading named {}", heading.name()),
            Self::Rpc(rpc) => format!("an rpc of {}", rpc.method),
        }
    }
}

impl PartialEq for Declaration {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Signal(one), Self::Signal(other)) => one == other,
            (Self::Property(one), Self::Property(other)) => one == other,
            (Self::Heading(one), Self::Heading(other)) => one == other,
            _ => false,
        }
    }
}

/// A file running now, the constants it has created so far, each as the
/// names of its namespace and its own, what its class and the classes inside
/// it have declared, by the path of the script each stands for, and the
/// digest of the source it runs.
struct Frame {
    path: String,
    constants: Vec<(Vec<String>, String)>,
    declarations: Vec<(String, Declaration)>,
    digest: u64,
    // The constants the file writes inside its own class, which exist only
    // once that class does.
    inner_constants: Vec<Vec<String>>,
    // The names its source's `export` and `signal` calls spell out, read
    // before it runs and published with what it declared.
    source_names: SourceNames,
}

/// The names a file's source spells out in its class body's `export` and
/// `signal` calls, and what each method it defines calls.
#[derive(Default)]
pub(super) struct SourceNames {
    pub exports: Vec<String>,
    pub signals: Vec<String>,
    pub method_calls: Vec<MethodCalls>,
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
    execute(mrb, path, prepare, settle, Run::Failed)
}

/// Runs the file at `path` again, with the source it has now, if it ran
/// cleanly; one that raised runs again as it did the first time, all or
/// nothing, and one that never ran is left to run when it is needed.
pub(super) fn run_again<T>(
    mrb: &Mrb,
    path: &str,
    prepare: impl FnOnce() -> Result<T, Error>,
    settle: impl FnOnce(T) -> Result<(), Error>,
) -> Result<(), Error> {
    let run = runs(mrb).files.borrow().get(path).copied();
    match run {
        Some(Run::Done) => execute(mrb, path, prepare, settle, Run::Done),
        Some(Run::Failed) => execute(mrb, path, prepare, settle, Run::Failed),
        Some(Run::Running) | None => Ok(()),
    }
}

/// Whether the file at `path` has run cleanly.
pub(super) fn has_run(mrb: &Mrb, path: &str) -> bool {
    matches!(runs(mrb).files.borrow().get(path), Some(Run::Done))
}

/// The files that have run cleanly.
pub(super) fn clean_run_paths(mrb: &Mrb) -> Vec<String> {
    runs(mrb)
        .files
        .borrow()
        .iter()
        .filter(|(_, run)| matches!(run, Run::Done))
        .map(|(path, _)| path.clone())
        .collect()
}

/// Runs the file at `path` for a name it defines, unless it has run cleanly
/// or is running now. A file that raised runs again, as a failed `require`
/// does in Ruby: it took away what it created, so it starts over.
pub(super) fn run_for_name<T>(
    mrb: &Mrb,
    path: &str,
    prepare: impl FnOnce() -> Result<T, Error>,
    settle: impl FnOnce(T) -> Result<(), Error>,
) -> Result<(), Error> {
    match runs(mrb).files.borrow().get(path) {
        Some(Run::Done | Run::Running) => return Ok(()),
        Some(Run::Failed) | None => {}
    }
    execute(mrb, path, prepare, settle, Run::Failed)
}

// Whether a file is running in the realm `mrb` belongs to.
pub(super) fn is_file_running(mrb: &Mrb) -> bool {
    !runs(mrb).frames.borrow().is_empty()
}

/// Whether the file at `path` is running now.
pub(super) fn is_running(mrb: &Mrb, path: &str) -> bool {
    matches!(runs(mrb).files.borrow().get(path), Some(Run::Running))
}

/// The files that led back to the one at `path` while it is still running,
/// ending with it; nothing when it is not running.
pub(super) fn trace_cycle(mrb: &Mrb, path: &str) -> Option<Vec<String>> {
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

/// Keeps `inner_constants`, the constants the file running now writes inside
/// its own class, until that class is created.
pub(super) fn keep_inner_constants(mrb: &Mrb, inner_constants: Vec<Vec<String>>) {
    if let Some(frame) = runs(mrb).frames.borrow_mut().last_mut() {
        frame.inner_constants = inner_constants;
    }
}

/// Keeps `names`, which the source of the file running now spells out, to
/// publish with what its class declares.
pub(super) fn keep_source_names(mrb: &Mrb, names: SourceNames) {
    if let Some(frame) = runs(mrb).frames.borrow_mut().last_mut() {
        frame.source_names = names;
    }
}

/// The constants the file running now writes inside its own class, once,
/// when `key` is that class's constant path.
pub(super) fn take_inner_constants(mrb: &Mrb, key: &[String]) -> Vec<Vec<String>> {
    let mut frames = runs(mrb).frames.borrow_mut();
    match frames.last_mut() {
        Some(frame) if key_by_path(mrb, &frame.path) == key => {
            std::mem::take(&mut frame.inner_constants)
        }
        _ => Vec::new(),
    }
}

/// Records that the file running now created `name` in the namespace the
/// names in `scope` spell.
pub(super) fn record_created(mrb: &Mrb, scope: Vec<String>, name: String) {
    if let Some(frame) = runs(mrb).frames.borrow_mut().last_mut() {
        frame.constants.push((scope, name));
    }
}

/// Records what `class`, whose file is running now, declares of itself. A
/// name is declared once: declaring it again as it stands is nothing new,
/// declaring it differently is refused where it is written, and a name an
/// ancestor declared is the ancestor's, refused in GDScript's own words so
/// that the same mistake reads the same in both languages.
pub(super) fn declare(mrb: &Mrb, class: RClass, declaration: Declaration) -> Result<(), Error> {
    if let Some(ancestor) = ancestor_by_member(mrb, class, declaration.name()) {
        return Err(make_ruby_error(
            mrb,
            c"ArgumentError",
            &format!(
                "The member \"{}\" already exists in parent class {ancestor}.",
                declaration.name()
            ),
        ));
    }
    let mut frames = runs(mrb).frames.borrow_mut();
    let Some(frame) = frames.last_mut() else {
        return Ok(());
    };
    let script = script_path_by_class(mrb, class, &frame.path);
    let Some((_, standing)) = frame
        .declarations
        .iter()
        .find(|(path, standing)| *path == script && standing.name() == declaration.name())
    else {
        frame.declarations.push((script, declaration));
        return Ok(());
    };
    if standing == &declaration {
        return Ok(());
    }
    let refusal = format!(
        "{} is already declared as {}, so it cannot be declared as {}",
        declaration.name(),
        standing.label(),
        declaration.label()
    );
    drop(frames);
    Err(make_ruby_error(mrb, c"ArgumentError", &refusal))
}

// The ancestor of `class` that declared `name`, if one did. Each Ruby
// superclass names a file, and what that file declared is published once it
// has run; the engine's own members are asked of the engine instead, so the
// walk stops at the engine class.
fn ancestor_by_member(mrb: &Mrb, class: RClass, name: &str) -> Option<String> {
    let snapshot = Arc::clone(&bookkeeping(mrb).snapshot.borrow());
    let mut current = class;
    loop {
        current = current.superclass(mrb)?;
        let path = current.path(mrb)?;
        if path.starts_with("Godot::") {
            return None;
        }
        let names: Vec<String> = path.split("::").map(str::to_owned).collect();
        let declared = file_by_constant(mrb, &names).is_some_and(|file| {
            snapshot
                .signals_by_path(&file)
                .iter()
                .any(|it| it.name == name)
                || snapshot.properties_by_path(&file).any(|it| it.name == name)
        });
        if declared {
            return Some(path);
        }
    }
}

/// Records `heading`, written by `class` in the file running now, which
/// names no member of the class, so it is written as often as the class
/// writes one. A heading written while no file runs belongs to no class.
pub(super) fn declare_heading(mrb: &Mrb, class: RClass, heading: Heading) {
    if let Some(frame) = runs(mrb).frames.borrow_mut().last_mut() {
        let script = script_path_by_class(mrb, class, &frame.path);
        frame
            .declarations
            .push((script, Declaration::Heading(heading)));
    }
}

// The path of the script `class` stands for while the file at `path` runs:
// the file's own, or for a class inside the file's class, the inner
// script's. Any other class the file writes declares for the file's class.
fn script_path_by_class(mrb: &Mrb, class: RClass, path: &str) -> String {
    let names: Vec<String> = class
        .path(mrb)
        .map(|written| written.split("::").map(str::to_owned).collect())
        .unwrap_or_default();
    match read_inner_names(mrb, path, &names) {
        Some(inner @ [_, ..]) => join_inner_path(path, inner),
        _ => path.to_owned(),
    }
}

// The paths of the scripts standing for the classes `frame` created inside
// its file's class, in the order it created them.
fn collect_inner_script_paths(mrb: &Mrb, frame: &Frame) -> Vec<String> {
    frame
        .constants
        .iter()
        .filter_map(|(scope, name)| {
            let inner = read_inner_names(mrb, &frame.path, scope)?;
            let mut names = inner.to_vec();
            names.push(name.clone());
            Some(join_inner_path(&frame.path, &names))
        })
        .filter(|path| {
            loader::constant_by_key(mrb, &key_by_path(mrb, path))
                .and_then(RClass::from_value)
                .is_some()
        })
        .collect()
}

// What `names` spells after the constant path of the file at `path`'s
// class, if it begins with that path.
fn read_inner_names<'n>(mrb: &Mrb, path: &str, names: &'n [String]) -> Option<&'n [String]> {
    let file_key = key_by_path(mrb, path);
    let is_inside = names.len() >= file_key.len()
        && names
            .iter()
            .zip(&file_key)
            .all(|(name, segment)| normalize(name) == *segment);
    is_inside.then(|| &names[file_key.len()..])
}

fn runs(mrb: &Mrb) -> &Runs {
    &bookkeeping(mrb).runs
}

// Runs the file at `path`, and leaves it as `on_raise` says when it raises:
// failed, with what it created taken away, or done, keeping all of it.
fn execute<T>(
    mrb: &Mrb,
    path: &str,
    prepare: impl FnOnce() -> Result<T, Error>,
    settle: impl FnOnce(T) -> Result<(), Error>,
    on_raise: Run,
) -> Result<(), Error> {
    let runs = runs(mrb);
    runs.files
        .borrow_mut()
        .insert(path.to_owned(), Run::Running);
    runs.frames.borrow_mut().push(Frame {
        path: path.to_owned(),
        constants: Vec::new(),
        declarations: Vec::new(),
        digest: 0,
        inner_constants: Vec::new(),
        source_names: SourceNames::default(),
    });
    let outcome = prepare().and_then(|prepared| {
        let source = source_by_path(mrb, path)?;
        if let Some(frame) = runs.frames.borrow_mut().last_mut() {
            frame.digest = snapshot::digest(&source);
        }
        let name =
            CString::new(path).map_err(|error| make_runtime_error(mrb, &error.to_string()))?;
        compile(mrb, &name, &bookkeeping(mrb).files.rewrite_source(&source))?;
        settle(prepared)
    });
    let frame = runs.frames.borrow_mut().pop();
    let run = match (&outcome, frame) {
        (Ok(()), frame) => {
            frame
                .into_iter()
                .for_each(|frame| publish_frame(mrb, frame));
            Run::Done
        }
        (Err(_), frame) => {
            if matches!(on_raise, Run::Failed) {
                frame.iter().for_each(|frame| take_away(mrb, frame));
            }
            on_raise
        }
    };
    runs.files.borrow_mut().insert(path.to_owned(), run);
    if !is_file_running(mrb) {
        methods::publish_changed(mrb);
    }
    outcome
}

// Publishes what the file `frame` ran declared: its class's, with the names
// its source spells out, and each class's inside it.
fn publish_frame(mrb: &Mrb, mut frame: Frame) {
    let inner_paths = collect_inner_script_paths(mrb, &frame);
    let declarations = split(take_declarations(&mut frame.declarations, &frame.path));
    methods::publish_class(
        mrb,
        &frame.path,
        declarations,
        frame.source_names,
        frame.digest,
    );
    for path in inner_paths {
        let declarations = split(take_declarations(&mut frame.declarations, &path));
        methods::publish_class(
            mrb,
            &path,
            declarations,
            SourceNames::default(),
            frame.digest,
        );
    }
}

// What `declarations` holds for the script at `path`, taken out of it.
fn take_declarations(
    declarations: &mut Vec<(String, Declaration)>,
    path: &str,
) -> Vec<Declaration> {
    let (script_declarations, others) = std::mem::take(declarations)
        .into_iter()
        .partition(|(script, _)| script == path);
    *declarations = others;
    script_declarations
        .into_iter()
        .map(|(_, declaration)| declaration)
        .collect()
}

// What a frame declared, as the kinds a class publishes: its signals, what
// the editor shows, which keeps the order it was declared in, and its rpcs.
fn split(declarations: Vec<Declaration>) -> DeclarationsByKind {
    let mut split = DeclarationsByKind::default();
    for one in declarations {
        match one {
            Declaration::Signal(signal) => split.signals.push(signal),
            Declaration::Property(property) => split.members.push(Member::Property(property)),
            Declaration::Heading(heading) => split.members.push(Member::Heading(heading)),
            Declaration::Rpc(rpc) => split.rpcs.push(rpc),
        }
    }
    split
}

/// What a class's body declared, by kind.
#[derive(Default)]
pub(super) struct DeclarationsByKind {
    pub signals: Vec<Signal>,
    pub members: Vec<Member>,
    pub rpcs: Vec<Rpc>,
}

// The source the realm's files give for the file at `path`.
fn source_by_path(mrb: &Mrb, path: &str) -> Result<String, Error> {
    bookkeeping(mrb)
        .files
        .source_by_path(path)
        .map_err(|why| make_runtime_error(mrb, &why))
}

// Removes what `frame` created, the last first, so a constant inside a class
// goes before the class.
fn take_away(mrb: &Mrb, frame: &Frame) {
    for (scope, name) in frame.constants.iter().rev() {
        if let Some(scope) = live_scope_by_names(mrb, scope) {
            scope.const_remove(mrb, name.as_str()).ok();
        }
    }
}

// The module the names in `scope` spell exactly, if each is still defined;
// asking for a missing one would load it by name.
fn live_scope_by_names(mrb: &Mrb, scope: &[String]) -> Option<Scope> {
    scope
        .iter()
        .try_fold(Scope::Class(mrb.object_class()), |outer, name| {
            let name = mrb.intern(name.as_bytes()).ok()?;
            if outer.const_defined_at(mrb, name) {
                outer.const_get(mrb, name).ok().and_then(Scope::from_value)
            } else {
                None
            }
        })
}

fn make_runtime_error(mrb: &Mrb, message: &str) -> Error {
    make_ruby_error(mrb, c"RuntimeError", message)
}

pub(super) fn make_ruby_error(mrb: &Mrb, class: &CStr, message: &str) -> Error {
    match mrb.exc_get(class) {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}
