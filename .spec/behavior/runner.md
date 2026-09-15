# Test runner

How a Ruby test suite runs headless: the runner scene the addon carries runs every test file under a directory in the game's realm and quits with whether the suite passed, so a project tests its Ruby without GDScript.

## Includes

- `tasks/support/ruby_tests.rb`
- `tasks/support/test_settings.rb`
- `godot/test/**/*.rb`

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

## `RT-003` A failure is reported at the assertion's Ruby line

| Step | Statement |
| --- | --- |
| Given | a test method whose assertion fails |
| When | the runner scene runs headless on its directory |
| Then | Godot's output names the test with the `res://` path and line of the failing assertion |

## `RT-004` Teardown runs after a test that failed

| Step | Statement |
| --- | --- |
| Given | a test class whose `teardown` prints, and a test method whose assertion fails |
| When | the runner scene runs headless on its directory |
| Then | what `teardown` prints appears in Godot's output |

## `RT-005` An exception other than an assertion fails the run

| Step | Statement |
| --- | --- |
| Given | a test method that raises an exception other than `Minitest::Assertion` |
| When | the runner scene runs headless on its directory |
| Then | the run fails |

## `RT-006` An exception in a test is reported where it was raised

| Step | Statement |
| --- | --- |
| Given | a test method that raises an exception other than `Minitest::Assertion` |
| When | the runner scene runs headless on its directory |
| Then | Godot's output names the test, the exception, and the `res://` path and line that raised it |

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

## `RT-010` Each test runs on a fresh instance after `setup`

| Step | Statement |
| --- | --- |
| Given | a test class whose `setup` sets an instance variable |
| Given | two test methods that each change it |
| When | the runner scene runs headless on their directory |
| Then | each test sees the value `setup` set rather than the other test's change |

## `RT-011` A failed `assert_equal` says what was expected and what it got

| Step | Statement |
| --- | --- |
| Given | an `assert_equal` whose expected and actual values differ |
| When | the assertion runs |
| Then | it raises `Minitest::Assertion` whose message shows the expected value before the actual one |

## `RT-012` Lifecycle hooks run around each test in minitest's order

| Step | Statement |
| --- | --- |
| Given | a test class whose lifecycle hooks, `setup`, test method and `teardown` each record their name |
| When | the test runs |
| Then | the names are recorded as `before_setup`, `setup`, `after_setup`, the test, `before_teardown`, `teardown`, `after_teardown` |

## `RT-013` A teardown step that raises does not stop the ones after it

| Step | Statement |
| --- | --- |
| Given | a test class whose `before_teardown` raises and whose `teardown` and `after_teardown` record their names |
| When | the test runs |
| Then | both names are recorded and the test reports the exception as an error |

## `RT-014` A message given to an assertion comes before the assertion's own

| Step | Statement |
| --- | --- |
| Given | an assertion that does not hold, given a message of its own |
| When | the assertion runs |
| Then | it raises `Minitest::Assertion` whose message is the given one followed by what the assertion expected |

## `RT-015` `assert_equal` refuses `nil` as the expected value

| Step | Statement |
| --- | --- |
| Given | an `assert_equal` whose expected value is `nil` |
| When | the assertion runs |
| Then | it raises `Minitest::Assertion` pointing to `assert_nil`, even when the actual value is `nil` too |

## `RT-016` A mock answers an expected call with its return value

| Step | Statement |
| --- | --- |
| Given | a `Minitest::Mock` expecting a call with an argument and a return value |
| When | the call is made with a matching argument |
| Then | it answers the return value |

## `RT-017` A mock refuses a call with arguments it did not expect

| Step | Statement |
| --- | --- |
| Given | a `Minitest::Mock` expecting a call with an argument |
| When | the call is made with an argument that does not match |
| Then | it raises `MockExpectationError` |

## `RT-018` `assert_mock` fails for an expected call the mock never received

| Step | Statement |
| --- | --- |
| Given | a `Minitest::Mock` expecting a call that is never made |
| When | `assert_mock` runs on it |
| Then | it raises `Minitest::Assertion` located at the test's own line |

## `RT-019` A stub replaces a method only within its block

| Step | Statement |
| --- | --- |
| Given | an object whose method is stubbed with a value |
| When | the method is called inside the block and again after it |
| Then | it answers the value inside the block and its own result after it |

## `RT-020` An exception in a test is reported with the calls that led to it

| Step | Statement |
| --- | --- |
| Given | a test method that calls a method raising an exception other than `Minitest::Assertion` |
| When | the runner scene runs headless on its directory |
| Then | Godot's output lists each Ruby frame from the raise up to the test method, and no frame of the test framework |

## `RT-021` What mruby says about a test file comes before the tests run

| Step | Statement |
| --- | --- |
| Given | a test file that does not parse |
| When | the runner scene runs headless on that directory |
| Then | Godot's output carries the file's script error before the run's summary |

## `RT-022` A failure appears in Godot's log at the assertion's Ruby line

| Step | Statement |
| --- | --- |
| Given | a test method whose assertion fails |
| When | the runner scene runs headless on its directory |
| Then | Godot's output carries an error naming the test and the failure at the `res://` path and line of the failing assertion |

## `RT-023` An exception in a test appears in Godot's log where it was raised

| Step | Statement |
| --- | --- |
| Given | a test method that raises an exception other than `Minitest::Assertion` |
| When | the runner scene runs headless on its directory |
| Then | Godot's output carries an error naming the test and the exception at the `res://` path and line that raised it |

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
| Then | Godot's output carries `Run options: --seed` and the seed |

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
