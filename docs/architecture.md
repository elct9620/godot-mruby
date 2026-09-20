# Architecture

How the parts of godot-mruby fit together. The words it uses are defined in `.spec/glossary.md`, and what each part promises is specified under `.spec/`.

## 1. Overview

### 1.1 Layers

```
┌───────────────────────────────────┐
│ Godot: the engine and the editor  │
└────────────────┬──────────────────┘
                 │ the contracts of a script language
┌────────────────▼──────────────────┐
│ Extension: Rust, through gdext    │
└────────────────┬──────────────────┘
                 │ realm::enter
┌────────────────▼──────────────────┐
│ Realm: where the game's Ruby runs │
└────────────────┬──────────────────┘
                 │ beni
┌────────────────▼──────────────────┐
│ mruby                             │
└───────────────────────────────────┘
```

Godot never sees Ruby. It talks to the extension through the contracts it gives every script language, so a `.rb` file is served wherever a `.gd` file is.

The extension keeps no Ruby state of its own: it turns what Godot asks into entries into the realm.

The realm is one `mrb_state`, reached only through `realm::enter`, and it reaches mruby only through beni. Calls flow downward, and the realm never turns back to Godot: the files it runs and the log it writes to are given to it as it opens.

### 1.2 What ships

```
res://addons/godot_mruby/
├─ godot_mruby.gdextension    names each platform's library
├─ bin/                       one library per platform
├─ runner.tscn                the test runner scene
├─ LICENSE
└─ THIRD_PARTY_LICENSES.txt   mruby and the shipped crates
```

What ships is the addon folder, placed where Godot expects a non-project asset: `addons/<name>/`, so its files clash with neither the project nor other assets.

The editor loads the library the `.gdextension` names for the platform, from Godot 4.6 on. Each library is a single binary: the crate links the mruby archive beni builds and the Prism parser `ruby-prism` builds, so nothing else is needed at run time, and macOS ships one universal library. Building needs libclang, which Prism's bindings are generated with.

mruby carries only the gems `build_config/mruby.rb` lists, and none that reaches the host: Ruby reaches files, sockets and processes through Godot.

### 1.3 Source layout

```
.
├─ rust/                   the extension crate
│  └─ src/                 one module per component (see 2.1)
├─ build_config/           mruby's build: gems, cross build
├─ godot/                  the project the extension is tested in
│  ├─ addons/godot_mruby/  the shipped addon
│  ├─ test/                the extension's tests, in Ruby
│  ├─ loader/              files the tests load by name
│  └─ verify/              scenes and runs godot:verify reads
├─ .spec/                  glossary, contract, behavior
├─ Rakefile, tasks/        task entry points
│  └─ support/             the tasks' logic
├─ .github/                the CI pipeline and its setup
└─ .claude/hooks/          edit-time format, stop-time gate
```

The extension is tested through itself: `godot/test/` holds Ruby tests of the extension, run by its own test runner. What a Ruby test cannot observe, `godot:verify` reads from the log of the scenes and runs under `verify/`. `loader/` holds the files those tests reach by name.

A `.rake` file is thin glue; what a task does lives in `tasks/support/`.

Build output stays out of the repository: `vendor/` holds mruby's source and archives, `rust/target/` the crate's builds, `bin/` the installed library, and `pkg/` the package.

## 2. Extension

### 2.1 Components

| Component | Godot counterpart |
| --- | --- |
| `RubyLanguage` in `language.rs`, its `Project` in `announcement.rs` | `ScriptLanguageExtension` |
| `ResourceFormatLoaderRubyScript` in `loader.rs` | `ResourceFormatLoader` |
| `RubyScript` in `script.rs`, answered from `parser.rs` and `ancestry.rs` | `ScriptExtension` |
| `RubyInstance` in `instance.rs` | A script instance |
| `RubyTestRunner` in `runner.rs` | A `Node` in `runner.tscn` |
| `settings.rs` | `ProjectSettings` under `mruby/` |
| `GameFiles`, `FilesOnDisk` in `game.rs` | `ResourceLoader` under `res://`, and the files on disk |
| `GodotLog` in `log.rs` | Godot's log |
| `Godot` in `bridge.rs` and `bridge/` | `ClassDB`, the engine's singletons, value types and utility functions |
| `RubyObject` in `bridge/ruby_object.rs` | A `RefCounted` standing for a Ruby object |
| `Snapshot` in `snapshot.rs` | What Godot asks a script about its class |

Each class Godot knows answers what Godot already asks of a script language; none opens another way for Godot to reach Ruby.

`ResourceFormatLoaderRubyScript` reads a file's source and runs nothing. The script answers Godot from its header, which Prism reads from that source, and its ancestry; only a node script makes an instance (2.4), and the language announces it (2.5).

The runner is an ordinary node the addon ships. `GameFiles`, `GodotLog` and the `Godot` gem are what the game's realm is given: the files under `res://`, Godot's log, and the engine (2.6). The names Godot knows are in `.spec/contract/godot.md`, and the settings in `.spec/contract/project_settings.md`.

### 2.2 Lifecycle

```
the library loads
  │
  ▼
Scene stage begins
  │  settings::register     mruby/test/* and their defaults
  │  language::register     RubyLanguage for .rb
  │  loader::register       .rb loads as RubyScript
  │  realm::prepare         GameFiles, GodotLog, the Godot gem
  ▼
the first entry
  │  the game's realm opens as prepared
  ▼
every frame
  │  realm::release_queued  objects of nodes freed since
  ▼
Scene stage ends
  │  loader::unregister
  │  language::unregister
  │  realm::close
  ▼
the library unloads
```

Everything registers at the Scene stage, the stage a script language has to be registered by; gdext registers the classes themselves.

Nothing opens the realm at startup: `lib.rs` only prepares how it opens. It opens at the first entry that needs Ruby, so a process that never runs a Ruby file never opens one.

Teardown runs in reverse: Godot stops loading `.rb` files and forgets the language before the realm closes.

### 2.3 Dependencies

```
lib.rs    registers Scripting and settings, prepares the realm
  │
  ▼
┌─ Scripting ───────────────────────────────┐   ┌─ Testing ────────────┐
│  loader ──► script ◄──► language          │   │  runner ──► minitest │
│               │            │              │   └──┬───────────────────┘
│               ▼            ▼              │      │
│            instance    announcement       │      │
└──┬────────────────────────────────────────┘      │
   │                                               │
   ▼                                               ▼
┌─ Given to the realm ──────────────────────────────────────┐
│  game ──► settings        bridge ──► log                  │
└──┬────────────────────────────────────────────────────────┘
   │
   ▼
┌─ Declarations ────────────────────────────┐
│  ancestry ──► parser                      │   Prism
└──┬────────────────────────────────────────┘
   ▼
┌─ Realm ───────────────────────────────────┐
│  realm                                    │
└─────────────────────┬─────────────────────┘
                      ▼
                   compiler
```

Each layer uses only the layers below it. The realm uses no other module but `compiler`: what it needs from Godot, `game` and `log` implement, and it neither parses a file nor knows the engine.

`parser` and `ancestry` read what a file declares without running it, naming files by `realm::key_of`, `realm::normalize` and `realm::file_named`. Scripting answers Godot from them; `game` hands the realm, through `Files::declared`, what each file opens and the superclass a node script's class is held to, asking `bridge` which engine classes are nodes. `language` and `script` refer to each other, as GDExtension script languages do; `minitest` uses the realm only for a failure's `Location`.

### 2.4 Node scripts

```
node.set_script ──► instance: recorded        no Ruby runs
                       │
                       │ first call of a method the header has
                       ▼
                    realm.build(node key) ──► made: initialize ──► built
                       │   held already (Player.new made it) ──► built
                       │   file still running, no class yet ──► recorded
                       │ the file or initialize raised: reported once
                       ▼
                    failed: calls nothing again

node freed (any thread) ──► release(key) ──► let go at the next entry or frame
```

The header and ancestry answer what Godot asks on any thread before the file runs: the engine node class it extends, and what its source writes out — methods, signals and exported values; once it has run, the snapshot answers for the class it became (2.7). A script makes an instance only for a node of that class.

The instance holds no Ruby value: the realm holds the node's Ruby object under the node's instance id, so a node Ruby made with `new` and one Godot made meet the same object. The instance goes to Godot through the extension interface rather than gdext's `ScriptInstance`, and a call copies what it needs before Ruby runs, since the engine may call back into the node or take its script away before Ruby returns. Freeing a node queues its key and never waits for the realm. The rules are in `.spec/behavior/script.md` and `.spec/behavior/held_objects.md`.

### 2.5 Announcement

```
the editor scans res://
  │  language.get_global_class_name(path), read from disk
  ▼
announcement::Project
  │  under a test directory        ──► not listed
  │  header + ancestry: no node    ──► not listed
  │  another node script's name    ──► not listed, warned of
  ▼
name     Boss                      the class's own name
base     Enemy                     nearest listed ancestor, or engine class
tool, abstract, icon               from the header
  │
  ▼
the editor's class list, saved for exported games
```

A node script is listed like a GDScript's `class_name`. The list is flat, as it is for C#, so a name drops its namespaces, and node scripts sharing one are none of them listed; the scan order never picks one.

Godot asks every language for its stack as it prints, so the language never prints while it answers: it reads the files from disk, as GDScript does, since a load that fails prints, and writes the warning by a deferred call. The rules are in `.spec/behavior/announcement.md`, and what the editor shows in `.spec/behavior/script.md`.

### 2.6 Bridge

```
Ruby                          bridge                                engine
node.position = v    ─►  EngineObject(Gd) ─► ClassDB setter ─►  the node
Godot::Vector2.new   ─►  EngineValue ─► the engine's variant calls
Godot.lerp(a, b, t)  ─►  one row of the utility table ─► godot::global
method(:hurt)        ─►  Callable holding a realm key ─────►  kept by the engine
any other object     ─►  RubyObject holding a realm key ───►  kept by the engine
◄── answers: data copied, objects shared; a script's node comes back as its Ruby object
```

The `Godot` gem is all Ruby sees of the engine. An engine object carries the engine's object itself, so a reference-counted one lives while Ruby holds it; a value type carries a copy and never changes. What Ruby hands the engine that the engine keeps, it keeps as a key, released when the engine lets go. What crosses, and how a refused call fails, is in `.spec/behavior/engine_classes.md`, `.spec/behavior/values.md` and `.spec/behavior/math.md`; what the instance relies on is in `.spec/contract/bridge.md`.

### 2.7 Snapshot

```
class body runs ──► its declarations, and the methods the class ends up with
                         │ the file's run commits
                         ▼
                    one immutable value, published
                         │ read by path: the script's own, then its ancestry
                         ▼
has_signal, get_property_list, has_method, and the default each property was exported with
```

A Ruby class takes its shape as its body runs, so what a class has cannot be read from the source alone. The realm publishes what a file's class has when that file's run commits, and everything answering Godot reads the published value rather than entering the realm, on whatever thread Godot asks from. A file that has not run is answered from its header instead, so a scene connects to a signal declared there before anything enters the realm, while a hint, a heading or a member metaprogramming defined waits for the run. A property's value is the node's own, held by its Ruby object: the instance keeps what Godot writes until that object exists, and answers from what it kept or from the published default. What a class declares is in `.spec/behavior/declarations.md`, and the published shape in `.spec/contract/snapshot.md`.

## 3. Realm

### 3.1 Entry

```
RubyInstance ─────┐   on a call of a method its class defines
RubyTestRunner ───┤   when it is ready
                  ▼
realm::enter(body)
  │  waits while another thread is inside; a thread inside goes on
  │  opens the realm at the first entry, as prepared
  ▼
body(&Realm)
  │  run(path)                    a file, once
  │  install::<G: Gem>()          an extension
  │  call(receiver, method, arg)  a constant's method
  │  build(path, key, make, args) an object of a file's class, held by key
  │  send(key, method, args)      a held object's method
  ▼
a Rust value, or a RubyError ──► RubyError::write, at its Ruby line
```

The realm is the one way into Ruby. A component hands `realm::enter` a body, and the body is given the realm, never its `mrb_state`.

What comes back is a Rust value, or a `RubyError` the component writes to a log; an exception's backtrace goes with it, answered as the language's stack while it is written, as `.spec/behavior/report.md` claims. Since nothing outside holds a Ruby value, the realm's inside changes without its callers changing.

There is one realm, the game's, entered by one thread at a time; that thread enters again when the engine calls back into scripts, up to 24 entries deep, which the smallest thread stack holds. Freeing a node never waits for it: the key goes into a queue that the next entry, or the next frame, empties. Its operations are specified in `.spec/contract/realm.md`.

### 3.2 Internals

```
realm.rs              Realm, prepare, enter, Files, Log, RubyError
│
├─ the mrb_state
│  └─ user data       the bookkeeping, private to realm/
│     ├─ files, log   what the realm was given as it opened
│     ├─ index, runs  the class index and each file's run
│     └─ registry     the objects held for keys outside
│
├─ reentrant.rs       the lock a thread inside takes again
├─ print.rs           puts, print and p write to the log
├─ constants.rs, .rb  const_missing and const_added hooks
├─ index.rs           the class index: constant path to file
├─ executor.rs        runs a file once, all or nothing
└─ registry.rs        keys to objects, rooted for the collector
```

A realm's bookkeeping sits in the state's user data. Held objects live in a hash rooted for the collector, so mruby never frees an object a node still uses. Ruby's calls back into Rust reach it from the state they are given, and no gem knows the slot.

What makes this mruby environment its own is defined as the realm opens: output to its log, and the hooks through which a missing constant asks the index and a running file's constants are recorded.

The index is built from the paths its files give, and the executor takes each file's source and declaration from them, so the realm never reads a file itself: what a file opens loads before it runs, and a node script's class is held to its declared superclass after. The loader's rules are in `.spec/behavior/loader.md`.

### 3.3 Extensions

```
Realm::open(files, log, extend)
  │  the realm's own: output, hooks
  │  extend(&realm)            install::<Godot>()     every game realm
  │  the class index           sees what extend defined
  ▼
RubyTestRunner
  │  realm.install::<Minitest>()                      only a test run
  ▼
Ruby in the realm sees Godot, and Minitest in a test run
```

A gem is an extension: beni's `Gem`, mruby's gem init convention. What every realm of a kind has is installed by its opener's `extend`, before the class index takes the files in, so a file naming one of its constants is warned about; what only one use needs is installed by whoever needs it, and the install warns of an indexed file naming what it added. Only the test runner installs `Minitest`, so a shipped game never has it.

A gem's methods run with the state, and reach the realm's bookkeeping only through the realm's functions that take it: holding an object under a key, finding it again, and asking the class index for a file.

The line between the two: what every realm is, its output and its hooks, belongs to the realm; what only some realms need is a gem. The framework is specified in `.spec/contract/minitest.md` and `.spec/behavior/minitest.md`.
