# Minitest

The Ruby a test is written against, spelled the way minitest spells it so a Ruby developer writes tests as they already do. It exists only in the interpreter the test runner prepares.

### What minitest has that this does not

- `assert_match` and `refute_match`: the extension's mruby has no `Regexp`.
- `assert_throws`: the extension's mruby has no `catch` and `throw`.
- `assert_output`, `assert_silent` and `capture_io`: Ruby prints to Godot's output, not to an IO a test can capture.
- `assert_path_exists` and `refute_path_exists`: Ruby reaches no file system of its own.
- `assert_pattern` and `refute_pattern`.
- A difference between two long values is shown as the expected and actual values, not as a diff.
- `Minitest::Mock` matches positional arguments only, delegates to nothing, cannot expect the Object methods it keeps, such as `to_s`, and has no `must_verify` expectation.
- `Object#stub` replaces only a method the object defines, and passes no keyword arguments.

## Includes

- `rust/src/**/*.rb`

## `Minitest::Test`

What a test class inherits: each public method whose name starts with `test_` is a test, run on its own instance.

```ruby
module Minitest
  class Test
  end
end
```

## `Minitest::Test::LifecycleHooks`

The steps around each test, in the order they run: `before_setup`, `setup`, `after_setup`, the test, then `before_teardown`, `teardown`, `after_teardown`. A test class overrides `setup` and `teardown`; the `before_` and `after_` hooks are for a library, which includes a module that overrides them and calls `super`. They live in a module rather than on `Minitest::Test` so such a module included into `Minitest::Test` itself still reaches them.

```ruby
module Minitest
  class Test
    module LifecycleHooks
    end
  end
end
```

## `Minitest::Test::LifecycleHooks#before_setup`

```ruby
module Minitest
  class Test
    module LifecycleHooks
      def before_setup
      end
    end
  end
end
```

## `Minitest::Test::LifecycleHooks#setup`

Runs before each test.

```ruby
module Minitest
  class Test
    module LifecycleHooks
      def setup
      end
    end
  end
end
```

## `Minitest::Test::LifecycleHooks#after_setup`

```ruby
module Minitest
  class Test
    module LifecycleHooks
      def after_setup
      end
    end
  end
end
```

## `Minitest::Test::LifecycleHooks#before_teardown`

Runs after each test, whether it passed or not, as do the two teardown steps after it.

```ruby
module Minitest
  class Test
    module LifecycleHooks
      def before_teardown
      end
    end
  end
end
```

## `Minitest::Test::LifecycleHooks#teardown`

```ruby
module Minitest
  class Test
    module LifecycleHooks
      def teardown
      end
    end
  end
end
```

## `Minitest::Test::LifecycleHooks#after_teardown`

```ruby
module Minitest
  class Test
    module LifecycleHooks
      def after_teardown
      end
    end
  end
end
```

## `Minitest::Assertion`

What a failed assertion raises; a test that raises it fails, where any other exception is an error.

```ruby
module Minitest
  class Assertion
  end
end
```

## `Minitest::Assertions`

What a test asserts with. Each assertion counts toward the run's assertions and raises `Minitest::Assertion` when what it asserts does not hold, with a message saying what it expected; a message given to it comes before that one.

```ruby
module Minitest
  module Assertions
  end
end
```

## `Minitest::Assertions#assert`

```ruby
module Minitest
  module Assertions
    def assert(test, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute`

```ruby
module Minitest
  module Assertions
    def refute(test, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_equal`

The expected value comes first, and it cannot be `nil`: that is what `assert_nil` asserts.

```ruby
module Minitest
  module Assertions
    def assert_equal(exp, act, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute_equal`

```ruby
module Minitest
  module Assertions
    def refute_equal(exp, act, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_nil`

```ruby
module Minitest
  module Assertions
    def assert_nil(obj, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute_nil`

```ruby
module Minitest
  module Assertions
    def refute_nil(obj, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_raises`

Returns the exception the block raised; the last argument may be a message instead of an exception class.

```ruby
module Minitest
  module Assertions
    def assert_raises(*exp)
    end
  end
end
```

## `Minitest::Assertions#flunk`

```ruby
module Minitest
  module Assertions
    def flunk(msg = nil)
    end
  end
end
```

## `Minitest::Assertions#skip`

Ends the test without failing it.

```ruby
module Minitest
  module Assertions
    def skip(msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_empty`

```ruby
module Minitest
  module Assertions
    def assert_empty(obj, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute_empty`

```ruby
module Minitest
  module Assertions
    def refute_empty(obj, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_in_delta`

Passes when `act` is no further than `delta` from `exp`.

```ruby
module Minitest
  module Assertions
    def assert_in_delta(exp, act, delta = 0.001, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute_in_delta`

```ruby
module Minitest
  module Assertions
    def refute_in_delta(exp, act, delta = 0.001, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_in_epsilon`

`assert_in_delta` with a delta of `epsilon` times the smaller magnitude of the two.

```ruby
module Minitest
  module Assertions
    def assert_in_epsilon(exp, act, epsilon = 0.001, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute_in_epsilon`

```ruby
module Minitest
  module Assertions
    def refute_in_epsilon(exp, act, epsilon = 0.001, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_includes`

```ruby
module Minitest
  module Assertions
    def assert_includes(collection, obj, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute_includes`

```ruby
module Minitest
  module Assertions
    def refute_includes(collection, obj, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_instance_of`

```ruby
module Minitest
  module Assertions
    def assert_instance_of(cls, obj, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute_instance_of`

```ruby
module Minitest
  module Assertions
    def refute_instance_of(cls, obj, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_kind_of`

```ruby
module Minitest
  module Assertions
    def assert_kind_of(cls, obj, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute_kind_of`

```ruby
module Minitest
  module Assertions
    def refute_kind_of(cls, obj, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_operator`

Sends `op` to `o1` with `o2`; without `o2` it is `assert_predicate`.

```ruby
module Minitest
  module Assertions
    def assert_operator(o1, op, o2 = UNDEFINED, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute_operator`

```ruby
module Minitest
  module Assertions
    def refute_operator(o1, op, o2 = UNDEFINED, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_predicate`

```ruby
module Minitest
  module Assertions
    def assert_predicate(o1, op, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute_predicate`

```ruby
module Minitest
  module Assertions
    def refute_predicate(o1, op, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_respond_to`

```ruby
module Minitest
  module Assertions
    def assert_respond_to(obj, meth, msg = nil, include_all: false)
    end
  end
end
```

## `Minitest::Assertions#refute_respond_to`

```ruby
module Minitest
  module Assertions
    def refute_respond_to(obj, meth, msg = nil, include_all: false)
    end
  end
end
```

## `Minitest::Assertions#assert_same`

Passes when both are the same object.

```ruby
module Minitest
  module Assertions
    def assert_same(exp, act, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#refute_same`

```ruby
module Minitest
  module Assertions
    def refute_same(exp, act, msg = nil)
    end
  end
end
```

## `Minitest::Assertions#pass`

Counts an assertion that always passes.

```ruby
module Minitest
  module Assertions
    def pass(_msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_mock`

Passes when the mock received every call it expected.

```ruby
module Minitest
  module Assertions
    def assert_mock(mock, msg = nil)
    end
  end
end
```

## `Minitest::Mock`

A stand-in object that answers the calls a test expects of it and nothing else: any other call raises `NoMethodError`.

```ruby
module Minitest
  class Mock
  end
end
```

## `Minitest::Mock#expect`

Expects one call to `name`, answered with `retval`. Each argument is matched with `===` or `==`, so a class matches any instance of it; a block given instead of arguments is called with them and must answer truthy. Expecting the same name again expects another call, answered in order.

```ruby
module Minitest
  class Mock
    def expect(name, retval, args = [], &blk)
    end
  end
end
```

## `Minitest::Mock#verify`

Raises `MockExpectationError` for the first expected call the mock did not receive.

```ruby
module Minitest
  class Mock
    def verify
    end
  end
end
```

## `MockExpectationError`

What a mock raises for a call it did not expect with those arguments, or an expected call it never received.

```ruby
class MockExpectationError
end
```

## `Object#stub`

Replaces the receiver's method `name` for the length of the block, then restores it. A callable replacement is called with the arguments; anything else is returned, calling the caller's block with `block_args` first.

```ruby
class Object
  def stub(name, val_or_callable, *block_args, &block)
  end
end
```

## `Minitest.run`

Runs every test class defined so far and answers the problems the run found, each as its message, then the file and line it happened at, or `nil` for both where neither is known; none means every test passed. The test runner calls it once the test files have run, and reports each problem through Godot's log.

| Attribute | Value |
| --- | --- |
| internal | yes |

```ruby
module Minitest
  def self.run(options = {})
  end
end
```
