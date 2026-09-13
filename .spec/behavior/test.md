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
