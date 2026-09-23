# Project settings

The settings the extension adds to a project, under `mruby/`. A project changes them in its Project Settings, so a renamed setting silently stops applying to every project that set it.

## Includes

- `rust/src/**/*.rs`

## Marker

`@setting`

## `mruby/test/directories`

The project's test directories, each searched recursively; `res://test` when a project sets none. The test runner runs the tests under them; an export leaves them out of the game, and an exported game's class index leaves them out too. `res://` itself cannot be one: the whole game is not a test directory.

## `mruby/test/pattern`

The file names under a test directory that are test files, matched as a glob; `*_test.rb` when a project sets none.
