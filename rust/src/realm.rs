use std::cell::{Cell, RefCell};
use std::ffi::CStr;
use std::sync::Mutex;

use beni::{Error, FromValue, Gem, IntoValue, Mrb, Value};

use crate::compiler;

mod constants;
mod executor;
mod index;
mod print;
mod registry;

use index::ClassIndex;
pub use index::{file_named, key_of, normalize};
pub use registry::Key;
use registry::Registry;

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
    /// What the file at `path` declares before it runs.
    fn declared(&self, path: &str) -> Declared;
}

/// What a file declares before it runs.
#[derive(Debug, Default)]
pub struct Declared {
    /// The constants its statements write, each as its names from the top
    /// level; the realm loads them before the statements open them.
    pub writes: Vec<Vec<String>>,
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
    registry: Registry,
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
// Keys let go of on any thread, waiting for the game's realm to take them.
static RELEASED: Mutex<Vec<Key>> = Mutex::new(Vec::new());

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
    realm.release_queued();
    body(realm)
}

/// Lets go of the object `key` holds at the realm's next entry or frame,
/// never waiting for the realm, so whatever frees a node never waits for Ruby.
pub fn release(key: Key) {
    RELEASED.lock().unwrap().push(key);
}

/// Lets go of the objects released keys hold, entering the game's realm only
/// when a key is waiting; the extension calls it every frame.
pub fn release_queued() {
    if RELEASED.lock().unwrap().is_empty() {
        return;
    }
    if let Some(realm) = GAME.lock().unwrap().as_ref() {
        realm.release_queued();
    }
}

/// Closes the game's realm; a key released for it holds nothing in the next.
pub fn close() {
    let mut game = GAME.lock().unwrap();
    game.take();
    RELEASED.lock().unwrap().clear();
}

impl Realm {
    /// Opens a realm that runs `files` and whose words go to `log`, first
    /// given by `extend` what every realm of its kind has, so the class
    /// index sees those names as it takes the files in.
    pub fn open(
        files: impl Files + 'static,
        log: impl Log + 'static,
        extend: impl FnOnce(&Realm) -> Result<(), RubyError>,
    ) -> Result<Self, RubyError> {
        let mut mrb = Mrb::open()
            .map_err(|error| RubyError::plain(format!("mruby did not open: {error}")))?;
        let paths = files.paths();
        let bookkeeping = Bookkeeping {
            files: Box::new(files),
            log: Box::new(log),
            index: RefCell::default(),
            runs: executor::Runs::default(),
            registry: Registry::new(&mrb),
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
        extend(&realm)?;
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

    /// Adds what `G` defines to what Ruby sees in this realm, refusing the
    /// indexed files that name what it added, as the index refuses a file
    /// naming a constant the realm already has.
    pub fn install<G: Gem>(&self) -> Result<(), RubyError> {
        let _scope = self.mrb.arena_scope();
        let bookkeeping = bookkeeping(&self.mrb);
        let missing: Vec<_> = bookkeeping
            .index
            .borrow()
            .file_keys()
            .into_iter()
            .filter(|key| constants::constant_at(&self.mrb, key).is_none())
            .collect();
        self.mrb
            .init_gem::<G>()
            .map_err(|error| RubyError::read(&self.mrb, None, &error))?;
        let added = missing
            .into_iter()
            .filter(|key| constants::constant_at(&self.mrb, key).is_some());
        bookkeeping
            .index
            .borrow_mut()
            .refuse_defined(added, bookkeeping.log.as_ref());
        Ok(())
    }

    /// Runs the file at `path` with the source the realm's files give, unless it
    /// has run in this realm already, whether it succeeded or not.
    pub fn run(&self, path: &str) -> Result<(), RubyError> {
        let _scope = self.mrb.arena_scope();
        executor::run(&self.mrb, path, || {
            constants::ensure_opened(&self.mrb, path)
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
        self.taken(answer, || {
            format!("{receiver}.{}", method.to_string_lossy())
        })
    }

    fn release_queued(&self) {
        let keys = std::mem::take(&mut *RELEASED.lock().unwrap());
        let registry = &bookkeeping(&self.mrb).registry;
        for key in keys {
            registry.release(&self.mrb, key);
        }
    }

    /// Runs the file at `path` once, and makes an object of the class its
    /// path names, held by the realm under the key this answers.
    pub fn build(&self, path: &str) -> Result<Key, RubyError> {
        self.run(path)?;
        let _scope = self.mrb.arena_scope();
        let class = constants::constant_at(&self.mrb, &key_of(path)).ok_or_else(|| {
            RubyError::plain(format!(
                "{path} has not defined the class its path names, so no object of it is built"
            ))
        })?;
        class
            .funcall(&self.mrb, c"new", &[])
            .and_then(|object| bookkeeping(&self.mrb).registry.hold(&self.mrb, object))
            .map_err(|error| RubyError::read(&self.mrb, Some(path), &error))
    }

    /// Calls `method` with `args` on the object `key` holds, and answers what
    /// it returned as a Rust value.
    pub fn send<A: IntoValue, R: FromValue>(
        &self,
        key: Key,
        method: &str,
        args: impl IntoIterator<Item = A>,
    ) -> Result<R, RubyError> {
        let _scope = self.mrb.arena_scope();
        let args: Vec<Value> = args
            .into_iter()
            .map(|arg| arg.into_value(&self.mrb))
            .collect();
        let answer = bookkeeping(&self.mrb)
            .registry
            .object(&self.mrb, key)
            .and_then(|object| {
                let name = self.mrb.intern(method.as_bytes())?;
                object.funcall(&self.mrb, name, &args)
            })
            .map_err(|error| RubyError::read(&self.mrb, None, &error))?;
        self.taken(answer, || format!("#{method}"))
    }

    // `answer` as the Rust value its caller takes, or why it is not one.
    fn taken<R: FromValue>(
        &self,
        answer: Value,
        called: impl FnOnce() -> String,
    ) -> Result<R, RubyError> {
        R::from_value(answer).ok_or_else(|| {
            RubyError::plain(format!(
                "{} answered {}, which is not what its caller takes",
                called(),
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

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::sync::{Mutex, MutexGuard};
    use std::thread;
    use std::time::Duration;

    use super::*;

    // The game's realm is one for the whole process, so its tests take turns.
    static TURN: Mutex<()> = Mutex::new(());

    const THING: &str = "res://thing.rb";

    struct Thing;

    impl Files for Thing {
        fn paths(&self) -> Vec<String> {
            vec![THING.to_owned()]
        }

        fn source(&self, _path: &str) -> Result<String, String> {
            Ok("class Thing\n  def answer\n    42\n  end\nend\n".to_owned())
        }

        fn declared(&self, _path: &str) -> Declared {
            Declared::default()
        }
    }

    struct Silent;

    impl Log for Silent {
        fn message(&self, _text: &str) {}
        fn raw(&self, _text: &str) {}
        fn record(&self, _level: Level, _at: Option<&Location>, _text: &str) {}
    }

    // A fresh game's realm holding a Thing, and the key it holds it for.
    fn held_thing() -> (MutexGuard<'static, ()>, Key) {
        let turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        close();
        prepare(|| Realm::open(Thing, Silent, |_| Ok(())));
        let key = enter(|realm| realm.build(THING)).unwrap_or_else(|_| panic!("a Thing is built"));
        (turn, key)
    }

    fn no_args() -> std::iter::Empty<Value> {
        std::iter::empty()
    }

    // @behavior RO-001
    #[test]
    fn a_released_key_no_longer_reaches_its_object() {
        let (_turn, key) = held_thing();

        release(key);
        let sent = enter(|realm| realm.send::<_, i64>(key, "answer", no_args()));

        assert!(sent.is_err());
    }

    // @behavior RO-002
    #[test]
    fn releasing_a_key_never_waits_for_the_realm() {
        let (_turn, key) = held_thing();
        let (inside, entered) = mpsc::channel();
        let (leave, left) = mpsc::channel::<()>();
        let holder = thread::spawn(move || {
            enter(|_| {
                inside.send(()).ok();
                left.recv().ok();
                Ok(())
            })
            .ok();
        });
        entered.recv().unwrap();

        let (released, done) = mpsc::channel();
        thread::spawn(move || {
            release(key);
            released.send(()).ok();
        });
        let returned = done.recv_timeout(Duration::from_secs(1)).is_ok();

        leave.send(()).unwrap();
        holder.join().unwrap();
        assert!(returned);
    }

    // @behavior RO-003
    #[test]
    fn a_frame_lets_go_of_released_keys_without_any_other_entry() {
        let (_turn, key) = held_thing();
        release(key);

        release_queued();

        let held = GAME
            .lock()
            .unwrap()
            .as_ref()
            .map(|realm| bookkeeping(&realm.mrb).registry.len(&realm.mrb));
        assert_eq!(held, Some(0));
    }
}
