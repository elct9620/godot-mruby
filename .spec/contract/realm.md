# Realm

The one way the extension runs Ruby. A component enters the game's realm and asks it to run a file, install an extension, or call a constant's method, and gets Rust values back; no component outside the realm holds its `mrb_state` or a Ruby value, so what the realm keeps inside can change without its callers changing. An extension's methods run inside the realm with its `mrb_state`, and reach the realm's own state only through the functions taking one. What a realm needs from its host it is given as it opens, so it never asks who the host is.

## Includes

- `rust/src/realm.rs`
- `rust/src/realm/index.rs`
- `rust/src/realm/registry.rs`

## `prepare`

Gives the game's realm the way it opens, which its first entry uses; a mod's realm is given its own the same way.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn prepare(open: impl Fn() -> Result<Realm, RubyError> + Send + 'static) {}
```

## `enter`

Runs the body inside the game's realm, one thread at a time and again on a thread already inside, opening the realm at its first entry the way it was prepared.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn enter<T>(body: impl FnOnce(&Realm) -> Result<T, RubyError>) -> Result<T, RubyError> {}
```

## `close`

Closes the game's realm unless the calling thread is inside it; the next entry opens a new one.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn close() {}
```

## `Roots`

The root directories a realm names its files from: `res://`, and those inside it whose files are named from the top level too. What reads files outside the realm is given the same roots the realm is, so it names each file as the realm does.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Roots;
```

## `Roots::new`

The root directories a list names besides `res://`, each written with or without its trailing slash; `res://` itself and a directory outside it add none.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Roots {
    pub fn new(directories: impl IntoIterator<Item = String>) -> Self {}
}
```

## `Roots::key_of`

The constant path a file's path names from the nearest root directory it sits under, one segment per namespace as the class index matches it.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Roots {
    pub fn key_of(&self, path: &str) -> Vec<String> {}
}
```

## `normalize`

A constant's name as the class index matches it against a path's segment: without underscores and in lower case.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn normalize(segment: &str) -> String {}
```

## `file_named`

The file a constant path written inside namespaces names, looked up among paths named from their root directories as the realm's loader looks it up: from the innermost namespace outward, with a name two files spell naming none. What reads files outside the realm finds the file the realm would run.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn file_named(paths: Vec<String>, roots: Roots, scope: &[String], names: &[String]) -> Option<String> {}
```

## `inside`

Whether this thread is inside the game's realm, which something outside asks before reaching for what only the realm can answer, since entering it from another thread waits for the one inside.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn inside() -> bool {}
```

## `hold`

Holds a Ruby object under a key in the realm an extension's method runs in, for an extension making an object something outside keeps the key for.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn hold(mrb: &Mrb, key: Key, object: Value) -> Result<(), Error> {}
```

## `hold_new`

Holds a Ruby object in the realm an extension's method runs in under a key the realm names, which no node's key ever is, for an extension handing the engine something that keeps only the key.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn hold_new(mrb: &Mrb, object: Value) -> Result<Key, Error> {}
```

## `held`

The Ruby object a key holds in the realm an extension's method runs in, if it holds one, so an object the realm holds for something outside reaches Ruby as itself.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn held(mrb: &Mrb, key: Key) -> Option<Value> {}
```

## `file_defining`

The file the class index of the realm an extension's method runs in names for a constant path, written from `Object`.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn file_defining(mrb: &Mrb, names: &[String]) -> Option<String> {}
```

## `declare_signal`

Takes a signal as declared by the class of the file running now in the realm an extension's method runs in, which the realm publishes once that file has run; a name a class or one of its ancestors declared already is refused, and a declaration made while no file runs belongs to no class and is not taken.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn declare_signal(mrb: &Mrb, class: RClass, signal: Signal) {}
```

## `declare_export`

Takes a property as exported by the class of the file running now in the realm an extension's method runs in, which the realm publishes once that file has run; a name a class or one of its ancestors declared already is refused, and a declaration made while no file runs belongs to no class and is not taken.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn declare_export(mrb: &Mrb, class: RClass, property: Property) {}
```

## `release`

Lets go of the object a key of the game's realm holds, at the realm's next entry or frame; it never waits for the realm, so a node is freed on any thread without waiting for Ruby.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn release(key: Key) {}
```

## `release_queued`

Enters the game's realm, if a key is waiting to be released, and lets go of what those keys hold; the extension calls it every frame.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn release_queued() {}
```

## `Realm`

Where the game's Ruby runs, handed only to the body of an entry.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Realm;
```

## `Realm::open`

Opens a realm that runs the files it is given and whose words go to the log it is given, extended by what every realm of its kind has before its class index takes the files in.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Realm {
    pub fn open(files: impl Files + 'static, log: impl Log + 'static, extend: impl FnOnce(&Realm) -> Result<(), RubyError>) -> Result<Self, RubyError> {}
}
```

## `Realm::run`

Runs the Ruby file at a path once, with the source the realm's files give for it.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Realm {
    pub fn run(&self, path: &str) -> Result<(), RubyError> {}
}
```

## `Realm::install`

Adds an extension - a beni gem - to what Ruby sees in the realm.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Realm {
    pub fn install<G: Gem>(&self) -> Result<(), RubyError> {}
}
```

## `Realm::call`

Calls a method on the constant a name spells, and answers what it returned as the Rust type asked for, converted as mruby converts a method's argument.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Realm {
    pub fn call<A: IntoValue, R: TryConvert>(&self, receiver: &str, method: &CStr, arg: A) -> Result<R, RubyError> {}
}
```

## `Key`

What something outside a realm keeps for the Ruby object the realm holds for it, rather than the object; the outside names it, so a node's key is the node's instance id and the node's Ruby object is found from the node alone.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Key;

impl From<i64> for Key {}
```

## `Realm::build`

Runs the Ruby file at a path once, and holds under a key the object a method makes with the arguments given when called on the class the path names, unless the key holds an object already or the file, still running, has not yet defined the class; answers which, so the caller initializes only the object it made and asks again for a class still to come.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Realm {
    pub fn build<A: IntoValue>(&self, path: &str, key: Key, make: &CStr, args: impl IntoIterator<Item = A>) -> Result<Built, RubyError> {}
}
```

## `Built`

What building an object for a key came to: made now, held already, or waiting for its file to define the class.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub enum Built {
    Made,
    Held,
    Waiting,
}
```

## `Realm::send`

Calls a method on the object a key holds, and answers what it returned as the Rust type asked for, converted as mruby converts a method's argument.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Realm {
    pub fn send<A: IntoValue, R: TryConvert>(&self, key: Key, method: &str, args: impl IntoIterator<Item = A>) -> Result<R, RubyError> {}
}
```

## `RubyError`

Why Ruby could not do what it was asked, read out of the realm as a value so a component writes it to the log.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct RubyError;
```

## `RubyError::write`

Writes the error to a log, at its Ruby line when it has one.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl RubyError {
    pub fn write(&self, log: &impl Log) {}
}
```

## `Files`

The Ruby files a realm runs: which there are, for its class index, and what each one's source is. A realm is given them as it opens, so it never knows where they are kept.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub trait Files {}
```

## `Files::paths`

Every file the class index takes in, by path.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub trait Files {
    fn paths(&self) -> Vec<String>;
}
```

## `Files::roots`

The root directories the class index names the files from; only `res://` for files that give none.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub trait Files {
    fn roots(&self) -> Roots;
}
```

## `Files::source`

The source of the file at a path, or why there is none.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub trait Files {
    fn source(&self, path: &str) -> Result<String, String>;
}
```

## `Files::declared`

What the file at a path declares before it runs, which the realm keeps as it runs the file.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub trait Files {
    fn declared(&self, path: &str) -> Declared;
}
```

## `Declared`

What a file declares before it runs: the constants its statements write, each as the names written from the top level, which the realm loads before the statements open them, and the superclass its class extends when something relies on it, which the realm holds the class to once the file has run.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Declared;
```

## `Extends`

The superclass a file's class is held to: the class another file's path names, or a constant no file names, spelled from the top level.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub enum Extends {}
```

## `Log`

Where a realm's words go: what Ruby prints, and the records placed at a Ruby file and line.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub trait Log {}
```

## `Log::message`

A line Ruby prints, as `puts` and `p` write one.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub trait Log {
    fn message(&self, text: &str);
}
```

## `Log::raw`

Text Ruby prints as it is, as `print` writes it.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub trait Log {
    fn raw(&self, text: &str);
}
```

## `Log::record`

An error, a warning or a script error, at a Ruby file and line when it has one.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub trait Log {
    fn record(&self, level: Level, at: Option<&Location>, text: &str);
}
```

## `Log::exception`

An exception, as a script error at the first frame of its backtrace, with the frames that name a line, most recent first.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub trait Log {
    fn exception(&self, text: &str, backtrace: &[Location]);
}
```

## `Level`

How serious a record is.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub enum Level {}
```

## `Location`

A line of a Ruby file, and the method it is in when it names one.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Location;
```
