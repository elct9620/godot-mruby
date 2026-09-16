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

The editor loads the library the `.gdextension` names for the platform, from Godot 4.6 on. Each library is a single binary: the crate links the mruby archive beni builds, so nothing else is needed at run time, and macOS ships one universal library.

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
| `RubyLanguage` in `language.rs` | `ScriptLanguageExtension` |
| `ResourceFormatLoaderRubyScript` in `loader.rs` | `ResourceFormatLoader` |
| `RubyScript` in `script.rs` | `ScriptExtension` |
| `RubyInstance` in `instance.rs` | A script instance |
| `RubyTestRunner` in `runner.rs` | A `Node` in `runner.tscn` |
| `settings.rs` | `ProjectSettings` under `mruby/` |
| `GameFiles` in `game.rs` | `ResourceLoader` under `res://` |
| `GodotLog` in `log.rs` | Godot's log |

Each class Godot knows answers what Godot already asks of a script language; none opens another way for Godot to reach Ruby.

`ResourceFormatLoaderRubyScript` reads a file's source and runs nothing. The script holds that source and answers Godot's questions without Ruby. The instance holds no Ruby state: each get, set, call or notification enters the realm, which runs its file the first time.

The runner is an ordinary node the addon ships. `GameFiles` and `GodotLog` are what the game's realm is given: the files under `res://`, and Godot's log. The names Godot knows are in `.spec/contract/godot.md`, and the settings in `.spec/contract/project_settings.md`.

### 2.2 Lifecycle

```
the library loads
  │
  ▼
Scene stage begins
  │  settings::register     mruby/test/* and their defaults
  │  language::register     RubyLanguage for .rb
  │  loader::register       .rb loads as RubyScript
  │  realm::prepare         GameFiles and GodotLog
  ▼
the first entry
  │  the game's realm opens as prepared
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
┌─ Scripting ───────────────────────────────┐
│  loader ──► script ◄──► language          │
│               │                           │
│               ▼                           │
│            instance                       │
└──┬────────────────────────────────────────┘
   │
   │   ┌─ Testing ────────────────────────────┐
   │   │  runner ──► minitest                 │
   │   └──┬───────────────────────────────────┘
   │      │
   │      ├──────► ┌─ Given to the realm ─────┐
   ├─────────────► │  log                     │
   │      │        │  game ──► settings       │
   │      │        └────────────┬─────────────┘
   │      │                     │
   ▼      ▼                     ▼
┌─ Realm ───────────────────────────────────┐
│  realm                                    │
└─────────────────────┬─────────────────────┘
                      ▼
                   compiler
```

Scripting serves `.rb` files to Godot, and Testing runs a project's Ruby tests; both reach Ruby only through the realm, by `instance` and `runner`.

The realm uses no other module but `compiler`. What it needs from Godot, `game` and `log` implement for it, so every arrow into the realm is a use of it and none leaves it for Godot.

Within Scripting, `language` and `script` refer to each other, as GDExtension script languages do through the language's singleton; `instance` is handed the language by its script and reaches down only. `minitest` uses the realm only for the `Location` it writes a failure at, and runs its own Ruby through `compiler`.

## 3. Realm

### 3.1 Entry

```
RubyInstance ─────┐   on get, set, call, notification
RubyTestRunner ───┤   when it is ready
                  ▼
realm::enter(body)
  │  waits while another thread is inside
  │  opens the realm at the first entry, as prepared
  ▼
body(&Realm)
  │  run(path)                    a file, once
  │  install::<G: Gem>()          an extension
  │  call(receiver, method, arg)  a constant's method
  ▼
a Rust value, or a RubyError ──► RubyError::write
```

The realm is the one way into Ruby. A component hands `realm::enter` a body, and the body is given the realm, never its `mrb_state`.

What comes back is a Rust value, or a `RubyError` the component writes to a log. Since nothing outside holds a Ruby value, the realm's inside changes without its callers changing.

There is one realm, the game's, entered by one thread at a time. Its operations are specified in `.spec/contract/realm.md`.

### 3.2 Internals

```
realm.rs              Realm, prepare, enter, Files, Log, RubyError
│
├─ the mrb_state
│  └─ user data       the bookkeeping, private to realm/
│     ├─ files, log   what the realm was given as it opened
│     └─ index, runs  the class index and each file's run
│
├─ print.rs           puts, print and p write to the log
├─ constants.rs, .rb  const_missing and const_added hooks
├─ index.rs           the class index: constant path to file
└─ executor.rs        runs a file once, all or nothing
```

A realm's bookkeeping sits in the state's user data: the files and log it was given, the class index and each file's run. Ruby's calls back into Rust reach it from the state they are given, and no gem knows the slot.

What makes this mruby environment its own is defined as the realm opens: output to its log, and the hooks through which a missing constant asks the index and a running file's constants are recorded.

The index is built from the paths its files give, and the executor takes each file's source from them, so the realm never reads a file itself. The loader's rules are in `.spec/behavior/loader.md`.

### 3.3 Extensions

```
RubyTestRunner
  │  realm.install::<Minitest>()
  ▼
Realm::install
  │  beni's Gem
  ▼
Minitest::init(&Mrb)
  │  compiler::run      minitest.rb, mock.rb, godot_plugin.rb
  │  LogReporter#error  defined to write a failed test to the log
  ▼
Ruby in the realm sees Minitest
```

A gem is an extension: beni's `Gem`, mruby's gem init convention, installed by whoever needs it. Only the test runner installs `Minitest`, so a shipped game never has it.

A gem is handed the state only inside its `init`, and never touches the realm's bookkeeping.

The line between the two: what every realm is, its output and its hooks, belongs to the realm; what only some realms need is a gem. The framework is specified in `.spec/contract/minitest.md` and `.spec/behavior/minitest.md`.
