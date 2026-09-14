# Realm

The one way the extension runs Ruby. A component enters the game's realm and asks it to run a file, install an extension, or call a constant's method, and gets Rust values back; nothing outside the realm holds its `mrb_state` or a Ruby value, so what the realm keeps inside can change without its callers changing.

## Includes

- `rust/src/realm.rs`

## `enter`

Runs the body inside the game's realm, one thread at a time, opening the realm at its first entry.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn enter<T>(body: impl FnOnce(&Realm) -> Result<T, RubyError>) -> Result<T, RubyError> {}
```

## `close`

Closes the game's realm; the next entry opens a new one.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn close() {}
```

## `Realm`

Where the game's Ruby runs, handed only to the body of an entry.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Realm;
```

## `Realm::run`

Runs the Ruby file at a path once, with the source Godot holds for it.

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

## `RubyError`

Why Ruby could not do what it was asked, read out of the realm as a value so a component writes it to Godot's log.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct RubyError;
```

## `RubyError::log`

Writes the error to Godot's log, at its Ruby line when it has one.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl RubyError {
    pub fn log(&self) {}
}
```
