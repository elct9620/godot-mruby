# Realm

The one way the extension runs Ruby. A component enters the game's realm and asks it to run a file, install an extension, or call a constant's method, and gets Rust values back; nothing outside the realm holds its `mrb_state` or a Ruby value, so what the realm keeps inside can change without its callers changing. What a realm needs from its host it is given as it opens, so it never asks who the host is.

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

## `key_of`

The constant path a file's path names, one segment per namespace as the class index matches it, so what reads a file outside the realm names it as the realm does.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn key_of(path: &str) -> Vec<String> {}
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

The file a constant path written inside namespaces names, looked up among paths as the realm's loader looks it up: from the innermost namespace outward, with a name two files spell naming none. What reads files outside the realm finds the file the realm would run.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn file_named(paths: Vec<String>, scope: &[String], names: &[String]) -> Option<String> {}
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

Calls a method on the constant a name spells, and answers what it returned as a Rust value.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Realm {
    pub fn call<A: IntoValue, R: FromValue>(&self, receiver: &str, method: &CStr, arg: A) -> Result<R, RubyError> {}
}
```

## `Key`

A Ruby object a realm holds for something outside it, which keeps the key rather than the object.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Key;
```

## `Realm::build`

Runs the Ruby file at a path once, and makes an object of the class its path names, held by the realm under the key it answers.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Realm {
    pub fn build(&self, path: &str) -> Result<Key, RubyError> {}
}
```

## `Realm::send`

Calls a method on the object a key holds, and answers what it returned as a Rust value.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Realm {
    pub fn send<A: IntoValue, R: FromValue>(&self, key: Key, method: &str, args: impl IntoIterator<Item = A>) -> Result<R, RubyError> {}
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

## `Level`

How serious a record is.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub enum Level {}
```

## `Location`

A line of a Ruby file.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Location;
```
