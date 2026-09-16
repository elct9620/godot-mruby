use std::cell::{Cell, RefCell};
use std::ffi::CStr;
use std::sync::Mutex;

use beni::{Error, FromValue, Gem, IntoValue, Mrb};

use crate::compiler;

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

/// How serious a record is.
#[derive(Clone, Copy)]
pub enum Level {
    Error,
    Warn,
    /// A Ruby file that does not parse, the way Godot reports a GDScript one.
    ScriptError,
}

/// A line of a Ruby file.
pub struct Location {
    pub file: String,
    pub line: u32,
}

/// The Ruby files a realm runs: which there are, for its class index, and
/// what each one's source is. A realm is given them as it opens, so it never
/// knows where they are kept.
pub trait Files: Send {
    /// Every file the class index takes in, by path.
    fn paths(&self) -> Vec<String>;
    /// The source of the file at `path`, or why there is none.
    fn source(&self, path: &str) -> Result<String, String>;
}

/// Where a realm's words go: what Ruby prints, and the records placed at a
/// Ruby file and line. A realm is given one as it opens, so it never knows
/// whose log it writes to.
pub trait Log: Send {
    /// A line Ruby prints, as `puts` and `p` write one.
    fn message(&self, text: &str);
    /// Text Ruby prints as it is, as `print` writes it.
    fn raw(&self, text: &str);
    /// An error, a warning or a script error, at a Ruby file and line when
    /// it has one.
    #[track_caller]
    fn record(&self, level: Level, at: Option<&Location>, text: &str);
}

/// What a realm keeps beside its `mrb_state`, in the state's user data so
/// Ruby's calls back into Rust reach it from the state they are given.
struct Bookkeeping {
    files: Box<dyn Files>,
    log: Box<dyn Log>,
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
        bookkeeping(mrb)
            .log
            .record(Level::Warn, Some(&at), &warning.message);
    })
}

type Opener = Box<dyn Fn() -> Result<Realm, RubyError> + Send>;

// The way the game's realm opens, given before anything enters it.
static OPENER: Mutex<Option<Opener>> = Mutex::new(None);
static GAME: Mutex<Option<Realm>> = Mutex::new(None);

/// Gives the game's realm the way it opens, which its first entry uses. A
/// mod's realm will be given its own the same way, keyed by the mod.
pub fn prepare(open: impl Fn() -> Result<Realm, RubyError> + Send + 'static) {
    *OPENER.lock().unwrap() = Some(Box::new(open));
}

/// Runs `body` inside the game's realm, opening the realm first, the way it
/// was prepared, if this is its first entry.
pub fn enter<T>(body: impl FnOnce(&Realm) -> Result<T, RubyError>) -> Result<T, RubyError> {
    let mut game = GAME.lock().unwrap();
    let realm = match game.as_mut() {
        Some(realm) => realm,
        None => {
            let opener = OPENER.lock().unwrap();
            let open = opener.as_ref().ok_or_else(|| {
                RubyError::plain("the game's realm was entered before it was prepared".to_owned())
            })?;
            game.insert(open()?)
        }
    };
    body(realm)
}

pub fn close() {
    GAME.lock().unwrap().take();
}

impl Realm {
    /// Opens a realm that runs `files` and whose words go to `log`.
    pub fn open(files: impl Files + 'static, log: impl Log + 'static) -> Result<Self, RubyError> {
        let mut mrb = Mrb::open()
            .map_err(|error| RubyError::plain(format!("mruby did not open: {error}")))?;
        let paths = files.paths();
        let bookkeeping = Bookkeeping {
            files: Box::new(files),
            log: Box::new(log),
            index: RefCell::default(),
            runs: executor::Runs::default(),
            defining_namespace: Cell::default(),
        };
        if mrb.set_user_data(bookkeeping).is_err() {
            return Err(RubyError::plain(
                "mruby opened holding user data".to_owned(),
            ));
        }
        print::define(&mrb)
            .and_then(|()| constants::define(&mrb))
            .map_err(|error| RubyError::read(&mrb, None, &error))?;
        let realm = Self { mrb };
        realm.index_files(paths);
        Ok(realm)
    }

    // Adds the files at `paths` to the class index, each named from `res://`.
    fn index_files(&self, paths: impl IntoIterator<Item = String>) {
        let _scope = self.mrb.arena_scope();
        let bookkeeping = bookkeeping(&self.mrb);
        bookkeeping.index.borrow_mut().add(
            paths,
            |key| constants::constant_at(&self.mrb, key).is_some(),
            bookkeeping.log.as_ref(),
        );
    }

    /// Adds what `G` defines to what Ruby sees in this realm.
    pub fn install<G: Gem>(&self) -> Result<(), RubyError> {
        self.mrb
            .init_gem::<G>()
            .map_err(|error| RubyError::read(&self.mrb, None, &error))
    }

    /// Runs the file at `path` with the source the realm's files give, unless it
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
/// be written to a log from anywhere.
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

    /// Writes the error to `log`, at its Ruby line when it has one.
    #[track_caller]
    pub fn write(&self, log: &impl Log) {
        log.record(self.level, self.at.as_ref(), &self.message);
    }
}
