# godot-mruby

A Godot 4 addon that lets a game's scripts be written in Ruby, run by an embedded mruby.

```ruby
class Enemy < Godot::Node2D
  signal :arrived
  export :speed, 200.0
  export :health, 3

  def _physics_process(delta)
    self.position = position + Godot::Vector2.new(speed * delta, 0)
  end
end
```

## Features

The addon registers Ruby as a script language, so a `.rb` file is served wherever a `.gd` file is.

| Area | What it gives |
|---|---|
| Scripts | `.rb` scripts for nodes, their inner classes, resources and objects, with exports, signals and rpcs |
| Ruby | mruby 4.0 with Ruby's common core extensions; files, time and processes through Godot |
| Engine | classes, singletons and values under `Godot`; `position.y = 5` writes back |
| GDScript | calls a Ruby class's `new` and methods on `self`, awaits its methods |
| Loading | files load by constant name, no `require` |
| Editor | highlighting, completion, lookup, help pages, templates, validation, test panel |
| Tests | Minitest runner, left out of exports |

## Requirements

Each platform ships one self-contained library.

| Item | Supported |
|---|---|
| Godot | 4.6 or later |
| macOS | universal (arm64, x86_64) |
| Linux | x86_64, arm64 |
| Windows | x86_64, arm64 |

## Installation

The package is `godot-mruby.zip`, attached to each [GitHub release](https://github.com/elct9620/godot-mruby/releases).

1. Download `godot-mruby.zip` from a release.
2. Open the editor's AssetLib tab and click Import.
3. Choose the zip and install `addons/godot_mruby/`.
4. Restart the editor, the path the addon is tested on.

## Usage

### Ruby Scripts

A script's class extends an engine class under `Godot` and is attached to a node like a GDScript one. A file's path names its class, as `res://enemy.rb` holds `Enemy` and `res://units/turret.rb` holds `Units::Turret`. Using the constant loads the file.

```ruby
class Units::Turret < Godot::Node2D
  export :range, 300.0

  def _ready
    print "Turret #{name} covers #{range}"
  end
end
```

### Tests

Tests are Minitest classes in files ending in `_test.rb` under `res://test`. Run them from the editor's test panel, or headless as CI does.

```ruby
class EnemyTest < Minitest::Test
  def test_an_enemy_starts_with_three_health
    enemy = Enemy.new
    add_child_autofree(enemy)

    assert_equal 3, enemy.health
  end
end
```

```bash
godot --headless --path . res://addons/godot_mruby/runner.tscn -- --dir res://test
```

### Project Settings

The addon adds these settings under `mruby/` in Project Settings.

| Setting | Default | Purpose |
|---|---|---|
| `mruby/test/directories` | `res://test` | where tests live |
| `mruby/test/pattern` | `*_test.rb` | which files are tests |
| `mruby/loader/root_directories` | none | more roots naming files from the top |

### Limitations

These are what a project meets today, each with what to do about it.

| Limit | What to do |
|---|---|
| An exported game ships its `.rb` files as source | keep secrets out of scripts |
| Breakpoints and stepping are not available | trace with `print`, or `push_error`, which carries Ruby's backtrace |
| An engine error inside a Ruby call carries no Ruby backtrace | find the call by the engine method the error names |
| Ruby and the engine call into each other at most 24 deep | defer a call with `call_deferred` |
| Completion infers no variable's type | write the name bare, or after `self.` or `Godot::` |
| The editor's indent type is one setting for every language | set Editor Settings > Text Editor > Behavior > Indent to spaces |
| Godot 4.7 may keep Plain Text for a `.rb` opened before the addon | choose Ruby once in the script editor's Edit > Syntax Highlighter |

Saving converts indentation to the editor's indent type while Convert Indent On Save is on, so a Ruby project sets spaces. The Plain Text choice is [godotengine/godot#121288](https://github.com/godotengine/godot/issues/121288), with a fix slated for Godot 4.8.

## Development

Building needs Ruby with Bundler, Rust through rustup, and libclang. `rust-toolchain.toml` pins the compiler.

```bash
bundle install
bundle exec rake beni:build       # fetch and build mruby into vendor/
bundle exec rake extension:build  # build the extension into the addon
bundle exec rake                  # lint, build, then godot:verify
```

[`docs/architecture.md`](docs/architecture.md) lays out how the parts fit, and [`.spec/`](.spec/) holds what each part promises.

## License

godot-mruby is released under the [Apache License 2.0](LICENSE).
