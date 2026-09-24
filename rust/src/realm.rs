use std::cell::{Cell, RefCell};
use std::ffi::CStr;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use beni::{
    Array, Error, FromValue, Gem, IntoValue, Mrb, RClass, ReprValue, Symbol, TryConvert, Value,
};

use crate::compiler;
use crate::snapshot::{self, Class, Heading, Member, Property, Signal, Snapshot};

mod constants;
mod executor;
mod index;
mod print;
mod reentrant;
mod registry;

use executor::Declaration;
use index::ClassIndex;
pub use index::{Roots, file_by_name, normalize};
use reentrant::ReentrantLock;
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
    /// Ruby that fails as a script: a file that does not parse, or an
    /// exception it raises, the way Godot reports a GDScript one.
    ScriptError,
}

/// A line of a Ruby file, and the method it is in when it names one.
#[derive(Clone)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub function: String,
}

/// The Ruby files a realm runs: which there are, for its class index, and
/// what each one's source is. A realm is given them as it opens, so it never
/// knows where they are kept.
pub trait Files: Send {
    /// Every file the class index takes in, by path.
    fn paths(&self) -> Vec<String>;
    /// The root directories the class index names those files from.
    fn roots(&self) -> Roots {
        Roots::default()
    }
    /// The source of the file at `path`, or why there is none.
    fn source(&self, path: &str) -> Result<String, String>;
    /// What the file at `path` declares before it runs.
    fn declarations(&self, path: &str) -> Declarations;
}

/// What a file declares before it runs.
#[derive(Debug, Default)]
pub struct Declarations {
    /// The constants its statements write, each as its names from the top
    /// level; the realm loads them before the statements open them.
    pub writes: Vec<Vec<String>>,
    /// The superclass its class extends when something relies on it; once the
    /// file has run, a class extending another raises `TypeError`.
    pub extends: Option<Extends>,
}

/// The superclass a file's class is held to.
#[derive(Debug)]
pub enum Extends {
    /// The class the file at this path names.
    File(String),
    /// A constant no file names, as its names from the top level.
    Constant(Vec<String>),
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
    /// An exception, a script error at the first frame of its backtrace,
    /// with the frames that name a line, most recent first.
    fn exception(&self, text: &str, backtrace: &[Location]) {
        self.record(Level::ScriptError, backtrace.first(), text);
    }
}

/// What a realm keeps beside its `mrb_state`, in the state's user data so
/// Ruby's calls back into Rust reach it from the state they are given.
struct Bookkeeping {
    files: Box<dyn Files>,
    log: Box<dyn Log>,
    index: RefCell<ClassIndex>,
    runs: executor::Runs,
    registry: Registry,
    // What the files that have run declare, published once each of them has
    // run, so what any thread reads is never half-declared.
    snapshot: RefCell<Arc<Snapshot>>,
    // Set while the realm defines a directory's module, which no file created.
    defining_namespace: Cell<bool>,
    // How the Ruby running now started, while any does.
    outermost: Cell<Option<Started>>,
}

// The realm's outermost Ruby while it runs, forgotten once it returns or
// panics; an entry inside it leaves it as it is.
struct Outermost<'a>(Option<&'a Cell<Option<Started>>>);

impl<'a> Outermost<'a> {
    fn start(outermost: &'a Cell<Option<Started>>, started: Started) -> Self {
        if outermost.get().is_some() {
            return Self(None);
        }
        outermost.set(Some(started));
        Self(Some(outermost))
    }
}

impl Drop for Outermost<'_> {
    fn drop(&mut self) {
        if let Some(outermost) = self.0 {
            outermost.set(None);
        }
    }
}

/// How the realm's outermost Ruby started. mruby runs a file's top level from
/// Rust in its base frame and leaves it there once the file has run, so under
/// a call from Rust a backtrace ends with a top level that finished.
#[derive(Clone, Copy)]
enum Started {
    Run,
    Call,
}

impl Bookkeeping {
    // Publishes what the class of the file at `path` has, now that the file
    // has run. Ruby itself asks Godot what a class has, so a file's class is
    // read back while the thread that ran it is still inside the realm.
    fn ran(&self, path: &str, class: Class) {
        let mut draft = self.snapshot.borrow_mut();
        Arc::make_mut(&mut draft).ran(path, class);
        snapshot::publish(Arc::clone(&draft));
    }
}

// What the class of the file at `path` has, now that the file has run: what
// its body declared, and the methods it defines, so a method metaprogramming
// defined is one the class has.
fn ran(mrb: &Mrb, path: &str, signals: Vec<Signal>, members: Vec<Member>) {
    let methods = methods_of(mrb, path);
    bookkeeping(mrb).ran(
        path,
        Class {
            signals,
            members,
            methods,
        },
    );
}

// The names of the methods the class the file at `path` names defines
// itself; none when the file named no class of its own.
fn methods_of(mrb: &Mrb, path: &str) -> Vec<String> {
    let Some(class) = constants::constant_at(mrb, &key_of(mrb, path)) else {
        return Vec::new();
    };
    let own = [false.into_value(mrb)];
    let methods = class
        .funcall(mrb, c"instance_methods", &own)
        .ok()
        .and_then(Array::from_value);
    methods
        .map(|methods| {
            methods
                .entries(mrb)
                .filter_map(Symbol::from_value)
                .filter_map(|name| name.name(mrb))
                .collect()
        })
        .unwrap_or_default()
}

// The constant path the file at `path` spells in `mrb`'s realm.
fn key_of(mrb: &Mrb, path: &str) -> index::Key {
    bookkeeping(mrb).index.borrow().key_of(path)
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
            function: String::new(),
        };
        bookkeeping(mrb)
            .log
            .record(Level::Warn, Some(&at), &warning.message);
    })
}

type Opener = Box<dyn Fn() -> Result<Realm, RubyError> + Send>;

// The way the game's realm opens, given before anything enters it.
static OPENER: Mutex<Option<Opener>> = Mutex::new(None);
// A thread inside the game's realm enters it again, since Godot calls back
// into scripts while the Ruby it started still runs.
static GAME: ReentrantLock<RefCell<Game>> = ReentrantLock::new(RefCell::new(Game::Closed));
// How many entries one thread nests before the next fails: every entry past
// the first is Ruby calling the engine calling Ruby, which takes about 12 KiB
// of a debug build's stack and 5 KiB of a release build's, and 24 of them
// stay within half of the 512 KiB the smallest thread stack holds.
const DEEPEST_ENTRY: usize = 24;
// Keys let go of on any thread, waiting for the game's realm to take them.
static RELEASED: Mutex<Vec<Key>> = Mutex::new(Vec::new());

// The way the game's realm opens, which an opener that panicked leaves as it
// was, so the lock is taken whether or not that poisoned it.
fn opener() -> MutexGuard<'static, Option<Opener>> {
    OPENER.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Gives the game's realm the way it opens, which its first entry uses. A
/// mod's realm will be given its own the same way, keyed by the mod.
pub fn prepare(open: impl Fn() -> Result<Realm, RubyError> + Send + 'static) {
    *opener() = Some(Box::new(open));
}

/// The game's realm, as far as it has come.
enum Game {
    Closed,
    Opening,
    Open(Realm),
}

/// Runs `body` inside the game's realm, opening the realm first, the way it
/// was prepared, if this is its first entry. A thread already inside enters
/// again, unless it is already as deep as its stack allows.
pub fn enter<T>(body: impl FnOnce(&Realm) -> Result<T, RubyError>) -> Result<T, RubyError> {
    let game = GAME.lock();
    if game.depth() > DEEPEST_ENTRY {
        return Err(RubyError::plain(format!(
            "SystemStackError: the game's realm is already entered {DEEPEST_ENTRY} deep on this thread"
        )));
    }
    if matches!(*game.borrow(), Game::Closed) {
        let opening = Opening::start(&game);
        opening.opened(open_game()?);
    }
    let game = game.borrow();
    let Game::Open(realm) = &*game else {
        return Err(RubyError::plain(
            "the game's realm was entered while it opens".to_owned(),
        ));
    };
    realm.release_queued();
    body(realm)
}

// The game's realm while it opens. It goes back to closed unless it opened,
// so an opener that fails or panics leaves the next entry to try again.
struct Opening<'a>(&'a RefCell<Game>);

impl<'a> Opening<'a> {
    fn start(game: &'a RefCell<Game>) -> Self {
        *game.borrow_mut() = Game::Opening;
        Self(game)
    }

    fn opened(self, realm: Realm) {
        *self.0.borrow_mut() = Game::Open(realm);
    }
}

impl Drop for Opening<'_> {
    fn drop(&mut self) {
        let mut game = self.0.borrow_mut();
        if matches!(*game, Game::Opening) {
            *game = Game::Closed;
        }
    }
}

// Opens the game's realm the way it was prepared, holding no borrow of it, so
// what it opens with may enter and be refused.
fn open_game() -> Result<Realm, RubyError> {
    let opener = opener();
    let open = opener.as_ref().ok_or_else(|| {
        RubyError::plain("the game's realm was entered before it was prepared".to_owned())
    })?;
    open()
}

/// Takes `signal` as declared by `class`, whose file is running now in the
/// realm `mrb` belongs to, refusing a name the class or one of its ancestors
/// declared already; a declaration made while no file runs belongs to no
/// class and is not taken.
pub fn declare_signal(mrb: &Mrb, class: RClass, signal: Signal) -> Result<(), Error> {
    executor::declare(mrb, class, Declaration::Signal(signal))
}

/// Takes `property` as exported by `class`, whose file is running now in the
/// realm `mrb` belongs to, refusing a name the class or one of its ancestors
/// declared already; a declaration made while no file runs belongs to no
/// class and is not taken.
pub fn declare_export(mrb: &Mrb, class: RClass, property: Property) -> Result<(), Error> {
    executor::declare(mrb, class, Declaration::Property(property))
}

/// Takes `heading` as written by the file running now in the realm `mrb`
/// belongs to: a heading names no member, so nothing is refused for having
/// been written before.
pub fn declare_heading(mrb: &Mrb, heading: Heading) {
    executor::declare_heading(mrb, heading);
}

/// Whether this thread is inside the game's realm. Something outside asks
/// before reaching for what only the realm can answer, since entering it
/// from another thread waits for the one inside.
pub fn inside() -> bool {
    GAME.held_here()
}

/// Holds `object` under `key` in the realm `mrb` belongs to, for an
/// extension making an object something outside keeps the key for.
pub fn hold(mrb: &Mrb, key: Key, object: Value) -> Result<(), Error> {
    bookkeeping(mrb).registry.hold(mrb, key, object)
}

/// Holds `object` in the realm `mrb` belongs to under a key the realm names,
/// which no node's key ever is.
pub fn hold_new(mrb: &Mrb, object: Value) -> Result<Key, Error> {
    bookkeeping(mrb).registry.hold_new(mrb, object)
}

/// The Ruby object `key` holds in the realm `mrb` belongs to, if it holds
/// one.
pub fn held(mrb: &Mrb, key: Key) -> Option<Value> {
    bookkeeping(mrb).registry.object(mrb, key).ok()
}

/// The file the class index of the realm `mrb` belongs to names for the
/// constant `names` spells from Object.
pub fn file_by_constant(mrb: &Mrb, names: &[String]) -> Option<String> {
    let key: Vec<String> = names.iter().map(|name| normalize(name)).collect();
    match bookkeeping(mrb).index.borrow().entry(&key)? {
        index::Entry::File(path) => Some(path),
        index::Entry::Namespace(_) => None,
    }
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
    let game = GAME.lock();
    if let Game::Open(realm) = &*game.borrow() {
        realm.release_queued();
    }
}

/// Closes the game's realm, unless this thread is inside it; a key released
/// for it holds nothing in the next.
pub fn close() {
    let game = GAME.lock();
    if game.is_nested() {
        return;
    }
    *game.borrow_mut() = Game::Closed;
    RELEASED.lock().unwrap().clear();
    snapshot::publish(Arc::default());
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
        let index = RefCell::new(ClassIndex::new(files.roots()));
        let bookkeeping = Bookkeeping {
            files: Box::new(files),
            log: Box::new(log),
            index,
            runs: executor::Runs::default(),
            registry: Registry::new(&mrb),
            snapshot: RefCell::default(),
            defining_namespace: Cell::default(),
            outermost: Cell::default(),
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

    // Adds the files at `paths` to the class index, each named from its root
    // directory.
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
        self.started_as(Started::Run, || {
            executor::run(
                &self.mrb,
                path,
                || constants::ensure_opened(&self.mrb, path),
                |extends| constants::keep_extends(&self.mrb, path, extends),
            )
            .map_err(|error| RubyError::read(&self.mrb, Some(path), &error))
        })
    }

    /// Calls `method` on the constant `receiver` names with `args`, and
    /// answers what it returned as `R`, converted as mruby converts an
    /// argument.
    pub fn call<A: IntoValue, R: TryConvert>(
        &self,
        receiver: &str,
        method: &CStr,
        args: impl IntoIterator<Item = A>,
    ) -> Result<R, RubyError> {
        let _scope = self.mrb.arena_scope();
        let args: Vec<Value> = args
            .into_iter()
            .map(|arg| arg.into_value(&self.mrb))
            .collect();
        let answer = self.started_as(Started::Call, || {
            self.mrb
                .object_class()
                .as_value()
                .const_get(&self.mrb, receiver)
                .and_then(|receiver| receiver.funcall(&self.mrb, method, &args))
                .map_err(|error| RubyError::read(&self.mrb, None, &error))
        })?;
        self.converted_answer(answer, || {
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

    /// Runs the file at `path` once, and holds under `key` the object `make`
    /// answers with `args` on the class its path names, unless `key` holds
    /// one already or the file, still running, has not defined the class
    /// yet. Answers which, so its caller initializes only what it made.
    pub fn build<A: IntoValue>(
        &self,
        path: &str,
        key: Key,
        make: &CStr,
        args: impl IntoIterator<Item = A>,
    ) -> Result<Built, RubyError> {
        self.run(path)?;
        let _scope = self.mrb.arena_scope();
        let registry = &bookkeeping(&self.mrb).registry;
        let read = |error| RubyError::read(&self.mrb, Some(path), &error);
        if registry.holds(&self.mrb, key).map_err(read)? {
            return Ok(Built::Held);
        }
        let Some(class) = constants::constant_at(&self.mrb, &key_of(&self.mrb, path)) else {
            if executor::running(&self.mrb, path) {
                return Ok(Built::Waiting);
            }
            return Err(RubyError::plain(format!(
                "{path} has not defined the class its path names, so no object of it is built"
            )));
        };
        let args: Vec<Value> = args
            .into_iter()
            .map(|arg| arg.into_value(&self.mrb))
            .collect();
        self.started_as(Started::Call, || {
            class
                .funcall(&self.mrb, make, &args)
                .and_then(|object| registry.hold(&self.mrb, key, object))
                .map(|()| Built::Made)
                .map_err(read)
        })
    }

    /// Calls `method` with `args` on the object `key` holds, and answers what
    /// it returned as `R`, converted as mruby converts an argument.
    pub fn send<A: IntoValue, R: TryConvert>(
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
        let answer = self.started_as(Started::Call, || {
            bookkeeping(&self.mrb)
                .registry
                .object(&self.mrb, key)
                .and_then(|object| object.funcall(&self.mrb, method, &args))
                .map_err(|error| RubyError::read(&self.mrb, None, &error))
        })?;
        self.converted_answer(answer, || format!("#{method}"))
    }

    // Runs `ruby`, which Rust starts; it is the outermost Ruby when none runs.
    fn started_as<T>(&self, started: Started, ruby: impl FnOnce() -> T) -> T {
        let _outermost = Outermost::start(&bookkeeping(&self.mrb).outermost, started);
        ruby()
    }

    // `answer` as the Rust value its caller takes, or why it is not one.
    fn converted_answer<R: TryConvert>(
        &self,
        answer: Value,
        called: impl FnOnce() -> String,
    ) -> Result<R, RubyError> {
        R::try_convert(answer, &self.mrb).map_err(|error| {
            RubyError::plain(format!(
                "{} answered {}: {}",
                called(),
                answer.inspect(&self.mrb),
                error.message(&self.mrb)
            ))
        })
    }
}

/// What building an object for a key came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Built {
    /// Made now, for its caller to initialize.
    Made,
    /// Held already, made by whoever made it.
    Held,
    /// Not made: its file is still running and has not defined the class.
    Waiting,
}

/// Why Ruby could not do what it was asked, read out of the realm so it can
/// be written to a log from anywhere.
pub struct RubyError {
    level: Level,
    message: String,
    at: Option<Location>,
    backtrace: Vec<Location>,
}

impl RubyError {
    fn plain(message: String) -> Self {
        Self {
            level: Level::Error,
            message,
            at: None,
            backtrace: Vec::new(),
        }
    }

    // A syntax error names its own line, the way Godot reports a script that
    // does not parse; an exception carries the frames of its backtrace that
    // name a line, as GDScript reports one raised at run time. Both read
    // through the realm they came from.
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
                    function: String::new(),
                }),
                backtrace: Vec::new(),
            },
            Error::Exception(_) => {
                let backtrace = raised_frames(mrb, error);
                if backtrace.is_empty() {
                    Self::plain(named(error.message(mrb)))
                } else {
                    Self {
                        level: Level::ScriptError,
                        message: error.message(mrb),
                        at: None,
                        backtrace,
                    }
                }
            }
            _ => Self::plain(named(error.to_string())),
        }
    }

    /// Writes the error to `log`, at its Ruby line when it has one.
    #[track_caller]
    pub fn write(&self, log: &(impl Log + ?Sized)) {
        if self.backtrace.is_empty() {
            log.record(self.level, self.at.as_ref(), &self.message);
        } else {
            log.exception(&self.message, &self.backtrace);
        }
    }
}

// The frames of `error`'s backtrace that name a line, most recent first.
// Under a call from Rust, the top level mruby's base frame kept from the last
// file it ran is not one Ruby called through, so it is left out.
fn raised_frames(mrb: &Mrb, error: &Error) -> Vec<Location> {
    let mut frames: Vec<Location> = error
        .backtrace(mrb)
        .iter()
        .filter_map(|frame| location_of(frame))
        .collect();
    let under_call = mrb
        .user_data::<Bookkeeping>()
        .is_some_and(|kept| matches!(kept.outermost.get(), Some(Started::Call)));
    if under_call && frames.last().is_some_and(|frame| frame.function.is_empty()) {
        frames.pop();
    }
    frames
}

// The place a backtrace frame names, as mruby writes one: `file:line`, then
// `:in method` when it is in one. A frame without a line, `(unknown):0`,
// places nothing.
fn location_of(frame: &str) -> Option<Location> {
    let (place, function) = frame.rsplit_once(":in ").unwrap_or((frame, ""));
    let (file, line) = place.rsplit_once(':')?;
    let line = line.parse().ok().filter(|&line| line > 0)?;
    Some(Location {
        file: file.to_owned(),
        line,
        function: function.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use std::panic;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex, MutexGuard};
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
            Ok(concat!(
                "class Thing\n",
                "  def answer = 42\n",
                "  def ratio = 1.5\n",
                "  def name = \"thing\"\n",
                "  def fail = raise(\"failed\")\n",
                "end\n"
            )
            .to_owned())
        }

        fn declarations(&self, _path: &str) -> Declarations {
            Declarations::default()
        }
    }

    const RAISING: &str = "res://raising.rb";

    // A file whose top level raises on its second line.
    struct Raising;

    impl Files for Raising {
        fn paths(&self) -> Vec<String> {
            vec![RAISING.to_owned()]
        }

        fn source(&self, _path: &str) -> Result<String, String> {
            Ok("ready = true\nraise \"raised at the top\"\n".to_owned())
        }

        fn declarations(&self, _path: &str) -> Declarations {
            Declarations::default()
        }
    }

    const SWORD: &str = "res://game/items/sword.rb";
    const SHIELD: &str = "res://mods/items/shield.rb";

    // One namespace under two root directories, and a file at the top of
    // `res://` asking for a constant from each.
    struct Armory;

    impl Files for Armory {
        fn paths(&self) -> Vec<String> {
            vec![
                SWORD.to_owned(),
                SHIELD.to_owned(),
                "res://probe.rb".to_owned(),
            ]
        }

        fn roots(&self) -> Roots {
            Roots::new(["res://game".to_owned(), "res://mods/".to_owned()])
        }

        fn source(&self, path: &str) -> Result<String, String> {
            Ok(match path {
                SWORD => "module Items\n  class Sword\n  end\nend\n",
                SHIELD => "module Items\n  class Shield\n  end\nend\n",
                _ => concat!(
                    "class Probe\n",
                    "  def self.armed(_) = [Items::Sword, Items::Shield].map(&:to_s)",
                    " == %w[Items::Sword Items::Shield] && !Object.const_defined?(:Game)\n",
                    "end\n"
                ),
            }
            .to_owned())
        }

        fn declarations(&self, _path: &str) -> Declarations {
            Declarations::default()
        }
    }

    // The frames an error carries, each as `file:line:method`.
    fn frames(error: Option<RubyError>) -> Vec<String> {
        error
            .map(|error| error.backtrace)
            .unwrap_or_default()
            .iter()
            .map(|frame| format!("{}:{}:{}", frame.file, frame.line, frame.function))
            .collect()
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
        let key = Key::from(1);
        enter(|realm| realm.build(THING, key, c"new", no_args()))
            .unwrap_or_else(|_| panic!("a Thing is built"));
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

    // @behavior RO-004
    #[test]
    fn an_answer_converts_to_what_its_caller_takes_as_an_argument_does() {
        let (_turn, key) = held_thing();

        let sent = enter(|realm| realm.send::<_, i64>(key, "ratio", no_args()));

        assert_eq!(sent.ok(), Some(1));
    }

    // @behavior RO-005
    #[test]
    fn an_answer_the_caller_cannot_take_names_the_call_and_why() {
        let (_turn, key) = held_thing();

        let sent = enter(|realm| realm.send::<_, i64>(key, "name", no_args()));

        let message = sent.err().map(|error| error.message);
        assert_eq!(
            message.as_deref(),
            Some("#name answered \"thing\": String cannot be converted to Integer")
        );
    }

    // @behavior RR-004
    #[test]
    fn a_raised_backtrace_holds_only_the_frames_ruby_called_through() {
        let (_turn, key) = held_thing();

        let sent = enter(|realm| realm.send::<_, i64>(key, "fail", no_args()));

        assert_eq!(frames(sent.err()), ["res://thing.rb:5:fail"]);
    }

    // @behavior RR-005
    #[test]
    fn a_backtrace_raised_by_a_files_top_level_after_a_call_holds_that_top_level() {
        let _turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        close();
        prepare(|| Realm::open(Raising, Silent, |_| Ok(())));

        let ran = enter(|realm| {
            realm.call::<_, bool>("Integer", c"===", [4_i64])?;
            realm.run(RAISING)
        });

        assert_eq!(frames(ran.err()), ["res://raising.rb:2:"]);
    }

    // @behavior RL-032
    #[test]
    fn one_namespace_spans_the_directories_spelling_it_under_different_root_directories() {
        let _turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        close();
        prepare(|| Realm::open(Armory, Silent, |_| Ok(())));

        let armed = enter(|realm| realm.call::<_, bool>("Probe", c"armed", [0_i64]));

        assert_eq!(armed.map_err(|e| e.message), Ok(true));
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

        let game = GAME.lock();
        let held = match &*game.borrow() {
            Game::Open(realm) => Some(bookkeeping(&realm.mrb).registry.len(&realm.mrb)),
            _ => None,
        };
        assert_eq!(held, Some(0));
    }

    // @behavior RE-001
    #[test]
    fn a_thread_inside_the_realm_enters_it_again() {
        let (_turn, key) = held_thing();

        let answer = enter(|_| enter(|realm| realm.send::<_, i64>(key, "answer", no_args())));

        assert_eq!(answer.ok(), Some(42));
    }

    // @behavior RE-006
    #[test]
    fn entries_nested_too_deep_on_one_thread_raise_system_stack_error() {
        let (_turn, key) = held_thing();
        fn nest(depth: usize, key: Key) -> Result<i64, RubyError> {
            enter(|realm| match depth {
                0 => realm.send::<_, i64>(key, "answer", no_args()),
                _ => nest(depth - 1, key),
            })
        }

        let deepest = nest(DEEPEST_ENTRY - 1, key);
        let beyond = nest(DEEPEST_ENTRY, key);

        assert_eq!(deepest.ok(), Some(42));
        assert!(beyond.is_err_and(|error| error.message.starts_with("SystemStackError")));
    }

    // @behavior RE-002
    #[test]
    fn another_thread_waits_until_the_outermost_entry_returns() {
        let (_turn, _key) = held_thing();
        let (inner_returned, returned) = mpsc::channel();
        let (leave, left) = mpsc::channel::<()>();
        let holder = thread::spawn(move || {
            enter(|_| {
                enter(|_| Ok(())).ok();
                inner_returned.send(()).ok();
                left.recv().ok();
                Ok(())
            })
            .ok();
        });
        returned.recv().unwrap();

        let (entered, inside) = mpsc::channel();
        let waiter = thread::spawn(move || {
            enter(|_| {
                entered.send(()).ok();
                Ok(())
            })
            .ok();
        });
        let entered_early = inside.recv_timeout(Duration::from_millis(200)).is_ok();
        leave.send(()).unwrap();
        let entered_after = inside.recv_timeout(Duration::from_secs(1)).is_ok();

        holder.join().unwrap();
        waiter.join().unwrap();
        assert!(!entered_early);
        assert!(entered_after);
    }

    // @behavior RE-003
    #[test]
    fn an_entry_made_while_the_realm_opens_fails() {
        let _turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        close();
        let (nested, outcome) = mpsc::channel();
        prepare(move || {
            let nested = nested.clone();
            Realm::open(Thing, Silent, move |_| {
                nested.send(enter(|_| Ok(())).is_err()).ok();
                Ok(())
            })
        });

        enter(|_| Ok(())).ok();

        assert_eq!(outcome.try_recv().ok(), Some(true));
    }

    // @behavior RE-005
    #[test]
    fn an_entry_after_the_realm_panicked_while_opening_opens_it_again() {
        let _turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        close();
        let opened = Arc::new(AtomicBool::new(false));
        let first = opened.clone();
        prepare(move || {
            if !first.swap(true, Ordering::SeqCst) {
                panic!("the opener panics the first time");
            }
            Realm::open(Thing, Silent, |_| Ok(()))
        });
        let panicked = panic::catch_unwind(|| enter(|_| Ok(()))).is_err();

        let entered = enter(|_| Ok(()));

        assert!(panicked);
        assert!(entered.is_ok());
    }

    // @behavior RE-004
    #[test]
    fn closing_the_realm_from_inside_it_leaves_it_open() {
        let (_turn, key) = held_thing();

        let answer = enter(|_| {
            close();
            enter(|realm| realm.send::<_, i64>(key, "answer", no_args()))
        });

        assert_eq!(answer.ok(), Some(42));
    }
}
