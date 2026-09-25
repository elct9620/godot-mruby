# Godot classes

The classes the extension registers with Godot. The editor and GDScript know them by name, so a renamed class breaks every project that refers to it.

## Includes

- `rust/src/**/*.rs`

## `RubyEditorPlugin`

The editor plugin the extension adds to every editor: it hands the editor the export plugin and the test panel as it opens.

```rust
struct RubyEditorPlugin;
```

## `RubyExportPlugin`

The export plugin that keeps a project's tests out of the game it exports.

```rust
struct RubyExportPlugin;
```

## `RubyLanguage`

The Ruby script language, registered with the engine for `.rb` files.

```rust
struct RubyLanguage;
```

## `RubyObject`

A Ruby object as the engine holds it: a reference-counted object standing for a Ruby object no engine type fits, which reaches Ruby again as that object and lets go of it once the engine drops the last reference.

```rust
struct RubyObject;
```

## `RubyScript`

The script a `.rb` file loads as, the way a `.gd` file loads as a `GDScript`.

```rust
struct RubyScript;
```

## `RubyTestPanel`

The editor's test panel, a dock that runs a project's Ruby tests through the runner scene and lists their results.

```rust
struct RubyTestPanel;
```

## `RubyTestRunner`

The node the addon's runner scene holds: it runs a Ruby test suite and quits with its outcome.

```rust
struct RubyTestRunner;
```
