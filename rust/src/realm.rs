use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::sync::Mutex;

use beni::{Ccontext, Error, FromValue, Gem, IntoValue, Mrb, Value};
use godot::classes::FileAccess;

use crate::log::{Level, Location};
use crate::output::Output;
use crate::settings;
use crate::{log, warn};

mod index;

use index::ClassIndex;

/// Where the game's Ruby runs: an `mrb_state`, the class index of the files
/// under `res://` and the paths of the files that have run. It opens at the
/// first entry, one thread at a time is inside it, and it hands out Rust
/// values rather than Ruby ones.
pub struct Realm {
    mrb: Mrb,
    index: RefCell<ClassIndex>,
    ran: RefCell<HashSet<String>>,
}

static GAME: Mutex<Option<Realm>> = Mutex::new(None);

/// Runs `body` inside the game's realm, opening the realm first if this is
/// its first entry.
pub fn enter<T>(body: impl FnOnce(&Realm) -> Result<T, RubyError>) -> Result<T, RubyError> {
    let mut game = GAME.lock().unwrap();
    let realm = match game.as_mut() {
        Some(realm) => realm,
        None => game.insert(Realm::open()?),
    };
    body(realm)
}

pub fn close() {
    GAME.lock().unwrap().take();
}

impl Realm {
    fn open() -> Result<Self, RubyError> {
        let mrb = Mrb::open()
            .map_err(|error| RubyError::plain(format!("mruby did not open: {error}")))?;
        let realm = Self {
            mrb,
            index: RefCell::new(ClassIndex::default()),
            ran: RefCell::new(HashSet::new()),
        };
        realm.install::<Output>()?;
        realm.index_files(index::game_files(&settings::test_directories()));
        Ok(realm)
    }

    /// Adds the files at `paths` to the class index, each named from `res://`.
    pub fn index_files(&self, paths: impl IntoIterator<Item = String>) {
        self.index.borrow_mut().add(paths, |key| self.defines(key));
    }

    // Whether the constant `key` spells is already here, matched the way the
    // class index matches it.
    fn defines(&self, key: &[String]) -> bool {
        let _scope = self.mrb.arena_scope();
        let mut scope = self.mrb.object_class().to_value(&self.mrb);
        for segment in key {
            match self.constant_matching(scope, segment) {
                Some(constant) => scope = constant,
                None => return false,
            }
        }
        true
    }

    // The constant `scope` holds whose name the class index matches to
    // `segment`.
    fn constant_matching(&self, scope: Value, segment: &str) -> Option<Value> {
        let constants = scope
            .funcall(&self.mrb, c"constants", &[])
            .and_then(|constants| constants.ensure_array(&self.mrb))
            .ok()?;
        let name = (0..constants.len())
            .map(|index| constants.entry(index as isize))
            .find(|name| index::normalize(&name.to_string(&self.mrb)) == segment)?;
        let name = name.to_sym(&self.mrb).ok()?.to_sym();
        scope.const_get(&self.mrb, name).ok()
    }

    /// Adds what `G` defines to what Ruby sees in this realm.
    pub fn install<G: Gem>(&self) -> Result<(), RubyError> {
        self.mrb
            .init_gem::<G>()
            .map_err(|error| RubyError::read(&self.mrb, None, &error))
    }

    /// Runs the file at `path` unless it has run in this realm already,
    /// whether it succeeded or not.
    pub fn run_file(&self, path: &str) -> Result<(), RubyError> {
        if !self.ran.borrow_mut().insert(path.to_owned()) {
            return Ok(());
        }
        if !FileAccess::file_exists(path) {
            return Err(RubyError::plain(format!("{path}: the file does not exist")));
        }
        let source = FileAccess::get_file_as_string(path).to_string();
        load(&self.mrb, path, &source)
            .map_err(|error| RubyError::read(&self.mrb, Some(path), &error))
    }

    /// Calls `method` on the constant `receiver` names with `arg`, and answers
    /// what it returned as a Rust value.
    pub fn call<A: IntoValue, R: FromValue>(
        &self,
        receiver: &str,
        method: &CStr,
        arg: A,
    ) -> Result<R, RubyError> {
        let _scope = self.mrb.arena_scope();
        let name = self.mrb.intern(receiver.as_bytes()).to_sym();
        let answer = self
            .mrb
            .object_class()
            .to_value(&self.mrb)
            .const_get(&self.mrb, name)
            .and_then(|receiver| receiver.funcall(&self.mrb, method, &[arg.into_value(&self.mrb)]))
            .map_err(|error| RubyError::read(&self.mrb, None, &error))?;
        R::from_value(answer).ok_or_else(|| {
            RubyError::plain(format!(
                "{receiver}.{} answered {}, which is not what its caller takes",
                method.to_string_lossy(),
                answer.inspect(&self.mrb)
            ))
        })
    }
}

/// Runs `source` as the file at `path`: mruby stamps the path on everything
/// compiled from it, so warnings, errors and backtraces name it. What the
/// compiler warns about is written to Godot's log at its line.
pub fn load(mrb: &Mrb, path: &str, source: &str) -> Result<(), Error> {
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

fn refused(mrb: &Mrb, message: &str) -> Error {
    match mrb.exc_get(c"RuntimeError") {
        Ok(class) => Error::new(mrb, class, message),
        Err(error) => error,
    }
}

/// Why Ruby could not do what it was asked, read out of the realm so it can
/// be written to Godot's log from anywhere.
pub struct RubyError {
    level: Level,
    message: String,
    at: Option<Location>,
}

impl RubyError {
    fn plain(message: String) -> Self {
        Self {
            level: Level::Error,
            message,
            at: None,
        }
    }

    // A syntax error names its own line, the way Godot reports a script that
    // does not parse; an exception renders only through the realm it was
    // raised in.
    fn read(mrb: &Mrb, path: Option<&str>, error: &Error) -> Self {
        let named = |message: String| match path {
            Some(path) => format!("{path}: {message}"),
            None => message,
        };
        match error {
            Error::Syntax(parse) => Self {
                level: Level::ScriptError,
                message: parse.message().to_owned(),
                at: path.map(|path| Location {
                    file: path.to_owned(),
                    line: parse.line().into(),
                }),
            },
            Error::Exception(_) => Self::plain(named(error.message(mrb))),
            _ => Self::plain(named(error.to_string())),
        }
    }

    #[track_caller]
    pub fn log(&self) {
        log!(self.level, at: self.at.as_ref(), "{}", self.message);
    }
}
