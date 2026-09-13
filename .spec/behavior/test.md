# Ruby tests

How a Ruby test suite runs headless: the runner scene the addon carries runs every test file under a directory in the game's interpreter and quits with whether the suite passed, so a project tests its Ruby without GDScript.

### What a test does not have yet

- A test reaches only what its own file and core Ruby define: files do not yet find each other by name.
- Test classes and their tests run in name order rather than a seeded random one.
- What mruby says about a test file, such as a parse error, is reported after the run's summary rather than where the file runs.

## Includes

- `tasks/support/godot.rb`
- `godot/test/**/*.rb`

## `RT-001` A test method of a test class runs

| Step | Statement |
| --- | --- |
| Given | a `*_test.rb` file under the test directory whose `Minitest::Test` subclass has a passing `test_` method |
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
| Given | a `*_test.rb` file under the test directory that does not parse |
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
