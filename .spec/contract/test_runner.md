# Test runner

The command line the runner scene answers, which is how CI runs a project's Ruby tests:

    godot --headless --path <project> res://addons/godot_mruby/runner.tscn -- <options>

It quits with 0 when every test passed, and with 1 when any failed, a test file did not load, or a test directory is not one it can run.

## Includes

- `rust/src/**/*.rs`

## Marker

`@option`

## `--dir`

The one test directory to run, which has to be one of `mruby/test/directories`; every one of them when it is not given.

## `--include`

Runs only the test methods named by the value, either by name alone or as `Class#name`; `-i` for short. A run it leaves without a test fails.

## `--exclude`

Leaves out the test methods named by the value, either by name alone or as `Class#name`; `-e` for short.

## `--seed`

The seed that orders the run's tests; `-s` for short, and a random one when it is not given. The run prints the seed it used as `Run options: --seed N`, so giving it again repeats the order.
