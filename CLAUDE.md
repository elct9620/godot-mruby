# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

godot-mruby is a Godot 4 addon that embeds mruby through a Rust GDExtension (godot-rust/gdext + beni).

## Index

| Path | Role |
|---|---|
| `rust/` | The extension crate (gdext, beni) |
| `godot/` | Integration-test Godot project |
| `godot/addons/godot_mruby/` | The shipped addon; `bin/` is build output |
| `build_config/mruby.rb` | mruby build config (host + x86_64 cross build) |
| `Rakefile`, `tasks/*.rake` | Task entry points |
| `tasks/support/` | Task logic: paths, library names, cargo env, Godot checks |
| `.claude/hooks/` | Edit-time formatting and lint; stop-time quality gate |

## Commands

```bash
bundle install
bundle exec rake beni:build              # download mruby into vendor/ and build its archives
bundle exec rake extension:build         # host debug build, installed into the addon's bin/
bundle exec rake godot:verify            # headless editor pass; fails unless the extension loaded
bundle exec rake extension:dist          # the library this platform ships (PROFILE=release|debug; macOS: lipo universal)
bundle exec rake addon:package           # zip the addon with every platform's library and third-party licenses
bundle exec rake addon:verify            # install the zip into a copy of godot/ and verify it loads
bundle exec rake                         # rubocop + extension:build + godot:verify
bundle exec rake beni:clean beni:build   # rebuild mruby after editing build_config/mruby.rb
```

`cargo` works directly from the repo root or `rust/`: `.cargo/config.toml` points it at `vendor/` and `rust-toolchain.toml` pins the compiler.

## Guidelines

- Run Godot only with `--headless`; never start anything that opens its GUI.
- Minimum Godot is 4.6: keep gdext's `api-4-6` feature and the `.gdextension`'s `compatibility_minimum` in step.
- Library file names are shared by `godot_mruby.gdextension` and `tasks/support/extension.rb`; change both together.
- Put task logic in `tasks/support/` (the only Ruby RuboCop checks) and keep `.rake` files as thin glue.
- beni skips `beni:build` while an archive exists, so a config change needs `beni:clean` first.
- Hooks format on edit and gate lint/tests at stop; formatting is settled before commit, not at stop.
- Commits follow Conventional Commits.
