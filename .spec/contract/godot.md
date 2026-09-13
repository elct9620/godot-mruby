# Godot classes

The classes the extension registers with Godot. The editor and GDScript know them by name, so a renamed class breaks every project that refers to it.

## Includes

- `rust/src/**/*.rs`

## `RubyLanguage`

The Ruby script language, registered with the engine for `.rb` files.

```rust
struct RubyLanguage;
```

## `RubyScript`

The script a `.rb` file loads as, the way a `.gd` file loads as a `GDScript`.

```rust
struct RubyScript;
```

## `RubyTestRunner`

The node the addon's runner scene holds: it runs a Ruby test suite and quits with its outcome.

```rust
struct RubyTestRunner;
```
