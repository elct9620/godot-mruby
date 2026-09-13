# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

godot-mruby is a Godot 4 addon that embeds mruby through a Rust GDExtension (godot-rust/gdext + beni).

## Index

| Path | Role |
|---|---|
| `rust/` | The extension crate (gdext, beni); `src/realm.rs` is where Ruby runs, and `src/minitest/` is the test framework the test runner installs into it |
| `godot/` | Integration-test Godot project; its `project.godot` lists the test directories |
| `godot/test/` | Ruby tests the test runner runs; `godot/order/` prints the order a seed gives, `godot/failing/` holds tests whose run must fail |
| `godot/loading/`, `godot/naming/` | Files tests load by name, and names the class index warns about |
| `godot/addons/godot_mruby/` | The shipped addon; `runner.tscn` is the test runner scene, `bin/` is build output |
| `build_config/mruby.rb` | mruby build config (host + x86_64 cross build) |
| `Rakefile`, `tasks/*.rake` | Task entry points |
| `tasks/support/` | Task logic: paths, library names, cargo env, Godot checks |
| `.spec/` | Specification verified by sumitsubo (`sumi`): glossary, contract, behavior |
| `.claude/hooks/` | Edit-time formatting and lint; stop-time quality gate |
| `.github/workflows/pipeline.yml` | Reusable check → build → package → integration → publish; `ci.yml` calls it |
| `.github/actions/` | Composite setup steps (Godot, Ruby/Rust/caches) |

## Commands

```bash
bundle install
bundle exec rake beni:build              # download mruby into vendor/ and build its archives
bundle exec rake extension:build         # host debug build, installed into the addon's bin/
bundle exec rake godot:verify            # headless editor pass, then the smoke and report scenes and the Ruby tests
godot --headless --path godot res://addons/godot_mruby/runner.tscn -- --dir res://test   # the Ruby tests alone
bundle exec rake extension:dist          # the library this platform ships (PROFILE=release|debug; macOS: lipo universal)
bundle exec rake addon:package           # zip the addon with every platform's library and third-party licenses
bundle exec rake addon:verify            # install the zip into a copy of godot/ and run godot:verify's checks on it
bundle exec rake                         # rubocop + extension:build + godot:verify
bundle exec rake beni:clean beni:build   # rebuild mruby after editing build_config/mruby.rb
sumi verify                              # check the source against .spec/
sumi fmt                                 # write .spec/ in sumi's form (--check only reports)
```

`sumi help glossary|contract|behavior` has the form of each specification.

`cargo` works directly from the repo root or `rust/`: `.cargo/config.toml` points it at `vendor/` and `rust-toolchain.toml` pins the compiler.

## Guidelines

- Run Godot only with `--headless`; never start anything that opens its GUI.
- Minimum Godot is 4.6: keep gdext's `api-4-6` feature and the `.gdextension`'s `compatibility_minimum` in step.
- `godot/.godot/extension_list.cfg` is committed so Godot loads the addon at startup; an editor that first discovers it and quits at once crashes (godotengine/godot#111048).
- Library file names are shared by `godot_mruby.gdextension` and `tasks/support/extension.rb`; change both together.
- Put task logic in `tasks/support/` (the only Ruby RuboCop checks) and keep `.rake` files as thin glue.
- beni skips `beni:build` while an archive exists, so a config change needs `beni:clean` first.
- Hooks format on edit and gate lint/tests at stop; formatting is settled before commit, not at stop.
- Commits follow Conventional Commits.
