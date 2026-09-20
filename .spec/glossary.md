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
- `docs/*.md`
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

A realm's map from constant paths to the files named after them, built from the files the realm is given by Zeitwerk's rules: every directory is a namespace, and a file names the constant its path spells, matched without underscores or case. The game's realm is given every file under `res://`, and an exported game leaves the test directories out.

### Loader

What stands in for `require` in a realm: the first time Ruby uses a constant the class index names, the loader runs the file that names it, so the file loads by name. Godot loading a file reads its source and runs nothing.

#### Rejected

- `autoload` - Godot's autoloads are singleton nodes, and Ruby's `Module#autoload` is seen by constant lookup; the loader answers only `const_missing`, which mruby calls once lookup has failed.

### Log

Where a run's words go, as one stream: messages - what Ruby prints and the run's report - and errors, warnings and script errors, each placed at a Ruby file and line. The editor shows it in its Output panel; a headless run prints it.

### Header

What a Ruby file says before it runs, read from its source without running it: the constants its `module` and `class` statements write, and of the class its path names, the name it is written with, the superclass written on its class statement and the namespaces that is looked up from, the names of the methods it defines, the signals its body declares, the properties it exports with a value its source writes out, and the `tool`, `abstract` and `icon` its body calls. Godot asks a script these on any thread, before its file has run, and a realm loads what the file writes before running it.

### Snapshot

What a realm has published of the classes its files define, for Godot to be answered from without entering it: for each file, what the class its path names declares and defines, gathered while the file runs and published, as one value that never changes, once that file has run. Godot asks a script its shape - the methods it has, the signals it declares - on any thread, so a busy realm never holds up the editor, a loading screen or an error report. A file that has not run has none, and its header answers instead.

#### Rejected

- `Reflection` - reflection asks the live class, which is inside the realm; a snapshot is what the realm left outside for anyone to read.

### Ancestry

The files a class inherits from through the superclasses their headers write, nearest first, and the engine class the last of them extends. Each superclass is found as the loader would find it, so the ancestry is known without running a file.

### Node script

A file whose class, the one its path names, extends an engine node class, itself or through its ancestry. It is the only kind of file a node takes as its script.

### Announcement

What a node script tells the editor to list it by, as `class_name` does for a GDScript: the name its class is written with, without its namespaces, the nearest announced class it inherits from or else its engine class, and its icon, `tool` and `abstract`. A file under a test directory is never announced, and neither is any of the node scripts sharing a name.

### Library file

A file that is not a node script: it loads by name and is never a node's script.

### Engine class

A class the engine registers, and the Ruby class under `Godot` that stands for it, made at its first use and inheriting as the engine's does, up to `Godot::Object`. A node script's class extends one, itself or through its ancestry.

### Engine object

An object of an engine class, which the engine and Ruby share: Ruby holds the engine's object itself, so the same object reaches either side as itself, and Ruby calls its methods, properties and signals by their names.

### Value type

One of the engine's own values that are neither Ruby's nor objects, such as `Vector2`, `Color` or `NodePath`: a class under `Godot::Value` whose values the engine builds and computes with, and which never change, since each side holds its own copy.
