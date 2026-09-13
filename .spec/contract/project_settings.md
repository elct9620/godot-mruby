# Project settings

The settings the extension adds to a project, under `mruby/`. A project changes them in its Project Settings, so a renamed setting silently stops applying to every project that set it.

## Includes

- `rust/src/**/*.rs`

## Marker

`@setting`

## `mruby/test/directories`

The directories whose tests the test runner runs, each searched recursively; `res://test` when a project sets none. `res://` itself cannot be one: the whole game is not a test directory.

## `mruby/test/pattern`

The file names under a test directory that are test files, matched as a glob; `*_test.rb` when a project sets none.
