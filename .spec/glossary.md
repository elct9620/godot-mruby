# Glossary

The words this project keeps, and the ones it turns down in their place.

## Godot mruby

### Includes

- `rust/src/**/*.rs`
- `rust/src/**/*.rb`
- `tasks/**/*.rb`
- `tasks/*.rake`
- `Rakefile`
- `build_config/*.rb`
- `CLAUDE.md`
- `.spec/**/*.md`

### Realm

Where a file's Ruby runs: one `mrb_state` and the bookkeeping the extension keeps beside it, entered by one thread at a time. A file's path chooses its realm, so every file under `res://` shares the game's, and nothing is shared between realms. What Ruby sees in a realm beyond mruby's core is what has been installed into it.

#### Rejected

- `Interpreter` - mruby is the interpreter; a realm is one world of it that the extension opens, enters and closes.
- `VM` - mruby's virtual machine is one part of a realm.
- `Sandbox` - a realm keeps names apart, not permissions: Ruby reaches the host through the engine.
- `World` - Godot's `World2D` and `World3D` are the spaces a viewport simulates and draws.

### Entry

A thread going into a realm to run Ruby: it waits while another thread is inside, and goes on while it is inside already. It stays inside from its outermost entry until that entry returns, engine calls made from Ruby included. A file runs at the first entry that needs it, never when Godot loads it.

### Test directory

A directory the test runner runs tests from: one of a project's `mruby/test/directories`, searched recursively.

### Test file

A file under a test directory whose name matches the project's `mruby/test/pattern`. Only the test runner runs one.

### Class index

A realm's map from constant paths to the files named after them, built from `res://` by Zeitwerk's rules: every directory is a namespace, and a file names the constant its path spells, matched without underscores or case. An exported game's index leaves the test directories out.

#### Rejected

- `autoload` - Godot's autoloads are singleton nodes, and Ruby's `Module#autoload` is seen by constant lookup; the class index answers only `const_missing`, which mruby calls once lookup has failed.
