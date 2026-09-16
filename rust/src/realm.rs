use std::cell::{Cell, RefCell};
use std::ffi::CStr;
use std::sync::Mutex;

use beni::{Error, FromValue, Gem, IntoValue, Mrb};
use godot::classes::Os;
use godot::obj::Singleton;

use crate::log;
use crate::log::{Level, Location};
use crate::settings;
use crate::{compiler, warn};

mod constants;
mod executor;
mod index;
mod print;

use index::ClassIndex;

/// Where the game's Ruby runs: an `mrb_state`, the bookkeeping kept beside
/// it, and the extensions installed into it. It opens at the first entry, one
/// thread at a time is inside it, and it hands out Rust values rather than
/// Ruby ones.
pub struct Realm {
    mrb: Mrb,
}

/// What a realm keeps beside its `mrb_state`, in the state's user data so
/// Ruby's calls back into Rust reach it from the state they are given.
#[derive(Default)]
struct Bookkeeping {
    index: RefCell<ClassIndex>,
    runs: executor::Runs,
    // Set while the realm defines a directory's module, which no file created.
    defining_namespace: Cell<bool>,
}

// The bookkeeping `mrb`'s realm put there as it opened.
fn bookkeeping(mrb: &Mrb) -> &Bookkeeping {
    mrb.user_data()
        .expect("a realm keeps its bookkeeping from the moment it opens")
}

// Compiles and runs `source` under `name` in the realm `mrb` belongs to,
// writing what the compiler warns about to the log at its line.
fn compile(mrb: &Mrb, name: &CStr, source: &str) -> Result<(), Error> {
    let file = name.to_string_lossy();
    compiler::run(mrb, name, source, |warning| {
        let at = Location {
            file: file.clone().into_owned(),
            line: warning.line,
        };
        warn!(at: &at, "{}", warning.message);
    })
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

// The directories the class index leaves out: an exported game's test
// directories. Tests run only on the editor's build, where every file is
// indexed so a test reaches any file by name.
fn left_out() -> Vec<String> {
    if Os::singleton().has_feature("template") {
        settings::test_directories()
    } else {
        Vec::new()
    }
}

impl Realm {
    fn open() -> Result<Self, RubyError> {
        let mut mrb = Mrb::open()
            .map_err(|error| RubyError::plain(format!("mruby did not open: {error}")))?;
        if mrb.set_user_data(Bookkeeping::default()).is_err() {
            return Err(RubyError::plain(
                "mruby opened holding user data".to_owned(),
            ));
        }
        print::define(&mrb)
            .and_then(|()| constants::define(&mrb))
            .map_err(|error| RubyError::read(&mrb, None, &error))?;
        let realm = Self { mrb };
        realm.index_files(index::game_files(&left_out()));
        Ok(realm)
    }

    // Adds the files at `paths` to the class index, each named from `res://`.
    fn index_files(&self, paths: impl IntoIterator<Item = String>) {
        let _scope = self.mrb.arena_scope();
        bookkeeping(&self.mrb).index.borrow_mut().add(paths, |key| {
            constants::constant_at(&self.mrb, key).is_some()
        });
    }

    /// Adds what `G` defines to what Ruby sees in this realm.
    pub fn install<G: Gem>(&self) -> Result<(), RubyError> {
        self.mrb
            .init_gem::<G>()
            .map_err(|error| RubyError::read(&self.mrb, None, &error))
    }

    /// Runs the file at `path` with the source Godot holds for it, unless it
    /// has run in this realm already, whether it succeeded or not.
    pub fn run(&self, path: &str) -> Result<(), RubyError> {
        let _scope = self.mrb.arena_scope();
        executor::run(&self.mrb, path, || {
            constants::ensure_namespaces(&self.mrb, path)
        })
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
        let answer = self
            .mrb
            .intern(receiver.as_bytes())
            .and_then(|name| {
                self.mrb
                    .object_class()
                    .to_value(&self.mrb)
                    .const_get(&self.mrb, name.to_sym())
            })
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
