# Test runner

The command line the runner scene answers, which CI and editors drive:

    godot --headless --path <project> res://addons/godot_mruby/runner.tscn -- <options>

It quits with 0 when every test passed, and with 1 when any failed, a test file did not load, or the test directory does not exist.

## Includes

- `rust/src/**/*.rs`

## Marker

`@option`

## `--dir`

The directory whose `*_test.rb` files run, searched recursively; `res://test` when it is not given.
