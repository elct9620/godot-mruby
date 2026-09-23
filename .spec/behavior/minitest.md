# Minitest

What a test written against the framework sees: its lifecycle, its assertions and its test doubles, each doing what minitest's does.

## Includes

- `godot/test/unit/minitest/**/*_test.rb`

## `RM-001` Each test runs on a fresh instance after `setup`

| Step | Statement |
| --- | --- |
| Given | a test class whose `setup` sets an instance variable |
| Given | two test methods that each change it |
| When | the runner scene runs headless on their directory |
| Then | each test sees the value `setup` set rather than the other test's change |

## `RM-002` Lifecycle hooks run around each test in minitest's order

| Step | Statement |
| --- | --- |
| Given | a test class whose lifecycle hooks, `setup`, test method and `teardown` each record their name |
| When | the test runs |
| Then | the names are recorded as `before_setup`, `setup`, `after_setup`, the test, `before_teardown`, `teardown`, `after_teardown` |

## `RM-003` A teardown step that raises does not stop the ones after it

| Step | Statement |
| --- | --- |
| Given | a test class whose `before_teardown` raises and whose `teardown` and `after_teardown` record their names |
| When | the test runs |
| Then | both names are recorded and the test reports the exception as an error |

## `RM-004` An assertion that holds raises nothing

| Step | Statement |
| --- | --- |
| Given | each assertion, given what it asserts |
| When | they run |
| Then | none of them raises |

## `RM-005` `pass` counts an assertion

| Step | Statement |
| --- | --- |
| Given | a test |
| When | it calls `pass` |
| Then | the test's count of assertions rises by one |

## `RM-006` A message given to an assertion comes before the assertion's own

| Step | Statement |
| --- | --- |
| Given | an assertion that does not hold, given a message of its own |
| When | the assertion runs |
| Then | it raises `Minitest::Assertion` whose message is the given one followed by what the assertion expected |

## `RM-007` A failed `assert_equal` says what was expected and what it got

| Step | Statement |
| --- | --- |
| Given | an `assert_equal` whose expected and actual values differ |
| When | the assertion runs |
| Then | it raises `Minitest::Assertion` whose message shows the expected value before the actual one |

## `RM-008` `assert_equal` refuses `nil` as the expected value

| Step | Statement |
| --- | --- |
| Given | an `assert_equal` whose expected value is `nil` |
| When | the assertion runs |
| Then | it raises `Minitest::Assertion` pointing to `assert_nil`, even when the actual value is `nil` too |

## `RM-009` `assert` fails on a falsy value

| Step | Statement |
| --- | --- |
| Given | a value that is `nil` or `false` |
| When | `assert` runs on it |
| Then | it raises `Minitest::Assertion` |

## `RM-010` `refute` fails on a truthy value

| Step | Statement |
| --- | --- |
| Given | a value other than `nil` and `false` |
| When | `refute` runs on it |
| Then | it raises `Minitest::Assertion` |

## `RM-011` `refute_equal` fails on equal values

| Step | Statement |
| --- | --- |
| Given | two equal values |
| When | `refute_equal` runs on them |
| Then | it raises `Minitest::Assertion` |

## `RM-012` `assert_nil` fails on a value other than `nil`

| Step | Statement |
| --- | --- |
| Given | `false` |
| When | `assert_nil` runs on it |
| Then | it raises `Minitest::Assertion` |

## `RM-013` `refute_nil` fails on `nil`

| Step | Statement |
| --- | --- |
| Given | `nil` |
| When | `refute_nil` runs on it |
| Then | it raises `Minitest::Assertion` |

## `RM-014` `assert_raises` answers the exception the block raised

| Step | Statement |
| --- | --- |
| Given | a block raising the exception class `assert_raises` is given |
| When | `assert_raises` runs it |
| Then | it answers the exception, with its message |

## `RM-015` `assert_raises` fails when the block raises nothing

| Step | Statement |
| --- | --- |
| Given | a block that raises nothing |
| When | `assert_raises` runs it with an exception class |
| Then | it raises `Minitest::Assertion` |

## `RM-016` `assert_raises` fails when the block raises another exception

| Step | Statement |
| --- | --- |
| Given | a block raising an exception of another class than `assert_raises` is given |
| When | `assert_raises` runs it |
| Then | it raises `Minitest::Assertion` |

## `RM-017` `flunk` fails

| Step | Statement |
| --- | --- |
| Given | a test |
| When | it calls `flunk` |
| Then | it raises `Minitest::Assertion` |

## `RM-018` `skip` ends the test with the message it is given

| Step | Statement |
| --- | --- |
| Given | a message |
| When | a test calls `skip` with it |
| Then | it raises `Minitest::Skip` carrying that message |

## `RM-019` `assert_empty` fails on a collection holding something

| Step | Statement |
| --- | --- |
| Given | a collection holding one element |
| When | `assert_empty` runs on it |
| Then | it raises `Minitest::Assertion` |

## `RM-020` `refute_empty` fails on an empty collection

| Step | Statement |
| --- | --- |
| Given | an empty string |
| When | `refute_empty` runs on it |
| Then | it raises `Minitest::Assertion` |

## `RM-021` `assert_in_delta` fails beyond the delta, saying how far apart the values are

| Step | Statement |
| --- | --- |
| Given | two numbers further apart than the delta |
| When | `assert_in_delta` runs on them |
| Then | it raises `Minitest::Assertion` whose message shows the difference against the delta |

## `RM-022` `refute_in_epsilon` fails within epsilon of the smaller magnitude

| Step | Statement |
| --- | --- |
| Given | two numbers whose difference is under epsilon times the smaller of them |
| When | `refute_in_epsilon` runs on them |
| Then | it raises `Minitest::Assertion` |

## `RM-023` `refute_includes` fails on a member

| Step | Statement |
| --- | --- |
| Given | a string and a part of it |
| When | `refute_includes` runs on them |
| Then | it raises `Minitest::Assertion` |

## `RM-024` `assert_instance_of` fails on an instance of another class, naming both

| Step | Statement |
| --- | --- |
| Given | an object and a class it is not an instance of |
| When | `assert_instance_of` runs on them |
| Then | it raises `Minitest::Assertion` whose message names the class expected and the object's own |

## `RM-025` `assert_kind_of` fails on an object of another kind

| Step | Statement |
| --- | --- |
| Given | an object and a class it is not a kind of |
| When | `assert_kind_of` runs on them |
| Then | it raises `Minitest::Assertion` |

## `RM-026` `assert_operator` fails when the operator answers false, naming it

| Step | Statement |
| --- | --- |
| Given | two operands the operator answers false for |
| When | `assert_operator` runs on them |
| Then | it raises `Minitest::Assertion` whose message names the operands and the operator |

## `RM-027` `assert_predicate` fails when the predicate answers false

| Step | Statement |
| --- | --- |
| Given | an object the predicate answers false for |
| When | `assert_predicate` runs on it |
| Then | it raises `Minitest::Assertion` |

## `RM-028` `assert_respond_to` fails for a method the object does not answer, naming its class

| Step | Statement |
| --- | --- |
| Given | an object and a method it does not respond to |
| When | `assert_respond_to` runs on them |
| Then | it raises `Minitest::Assertion` whose message names the object, its class and the method |

## `RM-029` `assert_same` fails on distinct objects, naming their ids

| Step | Statement |
| --- | --- |
| Given | two different objects |
| When | `assert_same` runs on them |
| Then | it raises `Minitest::Assertion` whose message names both objects with their object ids |

## `RM-030` A mock answers an expected call with its return value

| Step | Statement |
| --- | --- |
| Given | a `Minitest::Mock` expecting a call with an argument and a return value |
| When | the call is made with a matching argument |
| Then | it answers the return value |

## `RM-031` A mock matches an argument by its class

| Step | Statement |
| --- | --- |
| Given | a `Minitest::Mock` expecting a call with a class as its argument |
| When | the call is made with an instance of that class |
| Then | it answers the return value |

## `RM-032` A mock answers repeated expectations in the order they were made

| Step | Statement |
| --- | --- |
| Given | a `Minitest::Mock` expecting the same call twice, each with its own return value |
| When | the call is made twice |
| Then | the answers come in the order the expectations were made |

## `RM-033` A mock checks its arguments with the block it was given

| Step | Statement |
| --- | --- |
| Given | a `Minitest::Mock` expecting a call whose block accepts the arguments |
| When | the call is made with arguments the block accepts |
| Then | it answers the return value |

## `RM-034` A mock refuses a call with arguments it did not expect

| Step | Statement |
| --- | --- |
| Given | a `Minitest::Mock` expecting a call with an argument |
| When | the call is made with an argument that does not match |
| Then | it raises `MockExpectationError` |

## `RM-035` A mock refuses a call it does not expect at all

| Step | Statement |
| --- | --- |
| Given | a `Minitest::Mock` expecting nothing |
| When | a call is made on it |
| Then | it raises `NoMethodError` naming the method and the calls it expects |

## `RM-036` `assert_mock` fails for an expected call the mock never received

| Step | Statement |
| --- | --- |
| Given | a `Minitest::Mock` expecting a call that is never made |
| When | `assert_mock` runs on it |
| Then | it raises `Minitest::Assertion` located at the test's own line |

## `RM-037` A stub replaces a method only within its block

| Step | Statement |
| --- | --- |
| Given | an object whose method is stubbed with a value |
| When | the method is called inside the block and again after it |
| Then | it answers the value inside the block and its own result after it |

## `RM-038` A callable stub is called with the arguments

| Step | Statement |
| --- | --- |
| Given | an object whose method is stubbed with a lambda |
| When | the method is called inside the block |
| Then | it answers what the lambda answers for the arguments given |
