# Test runner

How a Ruby test suite runs headless: the runner scene the addon carries runs every test file under a directory in the game's realm and quits with whether the suite passed, so a project tests its Ruby without GDScript.

## Includes

- `tasks/support/ruby_tests.rb`
- `tasks/support/test_settings.rb`

## `RT-001` A test method of a test class runs

| Step | Statement |
| --- | --- |
| Given | a test file whose `Minitest::Test` subclass has a passing `test_` method |
| When | the runner scene runs headless on that directory |
| Then | the run passes and counts the test among its runs |

## `RT-002` A failed assertion fails the run

| Step | Statement |
| --- | --- |
| Given | a test method whose assertion fails |
| When | the runner scene runs headless on its directory |
| Then | the run fails |

## `RT-003` The run's report places a failure at the assertion's Ruby line

| Step | Statement |
| --- | --- |
| Given | a test method whose assertion fails |
| When | the runner scene runs headless on its directory |
| Then | the log carries a message naming the test with the `res://` path and line of the failing assertion |

## `RT-004` Teardown runs after a test that failed

| Step | Statement |
| --- | --- |
| Given | a test class whose `teardown` prints, and a test method whose assertion fails |
| When | the runner scene runs headless on its directory |
| Then | what `teardown` prints appears in the log |

## `RT-005` An exception other than an assertion fails the run

| Step | Statement |
| --- | --- |
| Given | a test method that raises an exception other than `Minitest::Assertion` |
| When | the runner scene runs headless on its directory |
| Then | the run fails |

## `RT-006` The run's report places an exception where it was raised

| Step | Statement |
| --- | --- |
| Given | a test method that raises an exception other than `Minitest::Assertion` |
| When | the runner scene runs headless on its directory |
| Then | the log carries a message naming the test, the exception, and the `res://` path and line that raised it |

## `RT-007` A test file that does not parse fails the run

| Step | Statement |
| --- | --- |
| Given | a test file that does not parse |
| When | the runner scene runs headless on that directory |
| Then | the run fails |

## `RT-008` A test directory that does not exist fails the run

| Step | Statement |
| --- | --- |
| Given | a test directory that does not exist |
| When | the runner scene runs headless on it |
| Then | the run fails |

## `RT-009` A skipped test does not fail the run

| Step | Statement |
| --- | --- |
| Given | a test directory whose tests pass, and a test method that calls `skip` |
| When | the runner scene runs headless on that directory |
| Then | the run passes |

## `RT-020` An exception in a test is reported with the calls that led to it

| Step | Statement |
| --- | --- |
| Given | a test method that calls a method raising an exception other than `Minitest::Assertion` |
| When | the runner scene runs headless on its directory |
| Then | the log lists each Ruby frame from the raise up to the test method, and no frame of the test framework |

## `RT-021` What mruby says about a test file comes before the tests run

| Step | Statement |
| --- | --- |
| Given | a test file that does not parse |
| When | the runner scene runs headless on that directory |
| Then | the log carries the file's script error before the run's summary |

## `RT-022` A failure is logged as an error at the assertion's Ruby line

| Step | Statement |
| --- | --- |
| Given | a test method whose assertion fails |
| When | the runner scene runs headless on its directory |
| Then | the log carries an error naming the test and the failure, placed at the `res://` path and line of the failing assertion |

## `RT-023` An exception in a test is logged as an error where it was raised

| Step | Statement |
| --- | --- |
| Given | a test method that raises an exception other than `Minitest::Assertion` |
| When | the runner scene runs headless on its directory |
| Then | the log carries an error naming the test and the exception, placed at the `res://` path and line that raised it |

## `RT-024` `--include` runs only the test it names

| Step | Statement |
| --- | --- |
| Given | a test directory holding several tests |
| When | the runner scene runs headless on it with `--include` naming one test as `Class#name` |
| Then | the run counts that one test among its runs and no other |

## `RT-025` `--exclude` leaves out the test it names

| Step | Statement |
| --- | --- |
| Given | a test directory whose one skipped test is the only one that skips |
| When | the runner scene runs headless on it with `--exclude` naming that test |
| Then | the run counts no skips |

## `RT-026` An `--include` that names no test fails the run

| Step | Statement |
| --- | --- |
| Given | a test directory none of whose tests has the name `--include` is given |
| When | the runner scene runs headless on it |
| Then | the run fails, saying nothing ran for that filter |

## `RT-027` A project that names no test directory runs the tests under `res://test`

| Step | Statement |
| --- | --- |
| Given | a project that sets neither `mruby/test/directories` nor `mruby/test/pattern` |
| When | the runner scene runs headless without `--dir` |
| Then | the run passes and counts the tests of the `*_test.rb` files under `res://test` |

## `RT-028` Only the files matching the project's test pattern run

| Step | Statement |
| --- | --- |
| Given | a project whose `mruby/test/pattern` matches one test file under its test directory |
| When | the runner scene runs headless on that directory |
| Then | the run counts only that file's tests |

## `RT-029` A `--dir` that is not one of the project's test directories fails the run

| Step | Statement |
| --- | --- |
| Given | a directory the project does not list in `mruby/test/directories` |
| When | the runner scene runs headless with `--dir` naming it |
| Then | the run fails, saying it is not a test directory of the project |

## `RT-030` `res://` itself cannot be a test directory

| Step | Statement |
| --- | --- |
| Given | the project's root `res://` |
| When | the runner scene runs headless with `--dir` naming it |
| Then | the run fails, saying `res://` cannot be a test directory |

## `RT-031` A run prints the seed it ordered its tests by

| Step | Statement |
| --- | --- |
| Given | a test directory |
| When | the runner scene runs headless on it without `--seed` |
| Then | the log carries `Run options: --seed` and the seed |

## `RT-032` The seed a run printed repeats its order

| Step | Statement |
| --- | --- |
| Given | the seed a run of a test directory printed |
| When | the runner scene runs headless on that directory with `--seed` giving it |
| Then | the tests run in the same order as in that run |

## `RT-033` A seed runs the tests out of name order

| Step | Statement |
| --- | --- |
| Given | a test directory whose test classes and methods are named in order |
| When | the runner scene runs headless on it with each of a handful of seeds |
| Then | at least one run runs its tests in an order other than their names' |

## `RT-034` A `--seed` that is not a whole number fails the run

| Step | Statement |
| --- | --- |
| Given | a `--seed` value that is not a whole number |
| When | the runner scene runs headless with it |
| Then | the run fails, saying `--seed` takes a whole number |
