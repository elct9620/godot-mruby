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

The realm is one `mrb_state`, reached only through `realm::enter`, and it reaches mruby only through beni. Calls flow downward; the realm turns back to Godot only for what Godot holds, a file's source, the listing of `res://` and the project's settings, and to write the log.

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

| Component | Godot contract |
| --- | --- |
| `RubyLanguage` in `language.rs` | `ScriptLanguageExtension` |
| `ResourceFormatLoaderRubyScript` in `loader.rs` | `ResourceFormatLoader` |
| `RubyScript` in `script.rs` | `ScriptExtension` |
| `RubyInstance` in `instance.rs` | A script instance |
| `RubyTestRunner` in `runner.rs` | A `Node` in `runner.tscn` |
| `settings.rs` | `ProjectSettings` under `mruby/` |

Each component answers what Godot already asks of a script language; none opens another way for Godot to reach Ruby.

The loader reads a file's source and runs nothing. The script holds that source and answers Godot's questions without Ruby. The instance holds no Ruby state: each get, set, call or notification enters the realm, which runs its file the first time.

The runner is an ordinary node the addon ships. The first four components make up Scripting, and the runner makes up Testing (see 2.3). The names Godot knows are in `.spec/contract/godot.md`, and the settings in `.spec/contract/project_settings.md`.

### 2.2 Lifecycle

```
the library loads
  │
  ▼
Scene stage begins
  │  settings::register     mruby/test/* and their defaults
  │  language::register     RubyLanguage for .rb
  │  loader::register       .rb loads as RubyScript
  ▼
the first entry
  │  the game's realm opens
  ▼
Scene stage ends
  │  loader::unregister
  │  language::unregister
  │  realm::close
  ▼
the library unloads
```

Everything registers at the Scene stage, the stage a script language has to be registered by; gdext registers the classes themselves.

Nothing opens the realm at startup. It opens at the first entry that needs Ruby, so a process that never runs a Ruby file never opens one.

Teardown runs in reverse: Godot stops loading `.rb` files and forgets the language before the realm closes.

### 2.3 Dependencies

```
lib.rs    registers Scripting and settings, closes realm
  │
  ▼
┌─ Scripting ───────────────────────────┐
│  loader ──► script ◄──► language      │
│               │            ▲          │
│               ▼            │          │
│            instance ───────┘          │
└───────────────┬───────────────────────┘
                │ instance
                ▼
┌─ Realm ───────────────────────────────┐
│  realm                                │◄─────┐
└───────┬───────────────────────────────┘      │
        │                                      │ runner
        │       ┌─ Testing ─────────────────┐  │
        │       │  runner ──────────────────┼──┘
        │       │    │                      │
        │       │    ▼                      │
        │       │  minitest                 │
        │       └────┬──────────────────────┘
        │            │
        ▼            ▼
┌───────────────────────────────────────┐
│  compiler                             │
│  log                                  │
│  settings                             │
└───────────────────────────────────────┘
```

The top-level modules fall into three contexts, much as `.spec/behavior/` is split: Scripting serves `.rb` files to Godot, Realm runs Ruby, and Testing runs a project's Ruby tests. The modules beneath belong to none: how any Ruby compiles, how any word reaches the log, and the project's settings.

`instance` and `runner` are the only ways into the realm, and the realm uses neither Scripting nor Testing.

Within Scripting, `language`, `script` and `instance` refer to one another, being one script language to Godot. `minitest` does not use the realm: a gem is handed an `mrb_state` only while it installs.

## 3. Realm

### 3.1 Entry

```
RubyInstance ─────┐   on get, set, call, notification
RubyTestRunner ───┤   when it is ready
                  ▼
realm::enter(body)
  │  waits while another thread is inside
  │  opens the realm at the first entry
  ▼
body(&Realm)
  │  run(path)                    a file, once
  │  install::<G: Gem>()          an extension
  │  call(receiver, method, arg)  a constant's method
  ▼
a Rust value, or a RubyError ──► RubyError::log
```

The realm is the one way into Ruby. A component hands `realm::enter` a body, and the body is given the realm, never its `mrb_state`.

What comes back is a Rust value, or a `RubyError` the component writes to the log. Since nothing outside holds a Ruby value, the realm's inside changes without its callers changing.

There is one realm, the game's, entered by one thread at a time. Its operations are specified in `.spec/contract/realm.md`.

### 3.2 Internals

```
realm.rs              Realm, enter, close, RubyError
│
├─ the mrb_state
│  └─ user data       the bookkeeping, private to realm/
│
├─ print.rs           puts, print and p write to the log
├─ constants.rs, .rb  const_missing and const_added hooks
├─ index.rs           the class index: constant path to file
└─ executor.rs        runs a file once, all or nothing
```

A realm's bookkeeping, the class index and each file's run, sits in the state's user data. Ruby's calls back into Rust reach it from the state they are given, and no gem knows the slot.

What makes this mruby environment its own is defined as the realm opens: output to Godot's log, and the hooks through which a missing constant asks the index and a running file's constants are recorded.

The executor takes a file's source from Godot's `ResourceLoader` and never reads a file itself. The loader's rules are in `.spec/behavior/loader.md`.

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
