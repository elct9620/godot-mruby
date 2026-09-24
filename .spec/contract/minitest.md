# Minitest

The Ruby a test is written against, spelled the way minitest spells it so a Ruby developer writes tests as they already do. It exists only in a realm the test runner installs it into.

### What minitest has that this does not

- `assert_match` and `refute_match`: the extension's mruby has no `Regexp`.
- A filter names a test exactly; the extension's mruby has no `Regexp` for a `/pattern/`.
- `assert_throws`: the extension's mruby has no `catch` and `throw`.
- `assert_output`, `assert_silent` and `capture_io`: Ruby prints to the log, not to an IO a test can capture.
- `assert_path_exists` and `refute_path_exists`: Ruby reaches no file system of its own.
- `assert_pattern` and `refute_pattern`.
- A difference between two long values is shown as the expected and actual values, not as a diff.
- `Minitest::Mock` matches positional arguments only, delegates to nothing, cannot expect the Object methods it keeps, such as `to_s`, and has no `must_verify` expectation.
- `Object#stub` replaces only a method the object defines, and passes no keyword arguments.
- Plugins are not found among installed gems, and none takes options: the one plugin is the extension's own, which writes each test that did not pass to the log.

## Includes

- `rust/src/**/*.rb`

## `Minitest::Test`

What a test class inherits: each public method whose name starts with `test_` is a test.

```ruby
module Minitest
  class Test
  end
end
```

## `Minitest::Test::LifecycleHooks`

The steps around each test. A test class overrides `setup` and `teardown`; the `before_` and `after_` hooks are for a library, which includes a module that overrides them and calls `super`. They live in a module rather than on `Minitest::Test` so such a module included into `Minitest::Test` itself still reaches them.

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

Runs after each test.

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

What a failed assertion raises.

```ruby
module Minitest
  class Assertion
  end
end
```

## `Minitest::Assertions`

What a test asserts with.

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

The expected value comes first.

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

The last argument may be a message instead of an exception class.

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

Without `o2` it is `assert_predicate`.

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

```ruby
module Minitest
  module Assertions
    def pass(_msg = nil)
    end
  end
end
```

## `Minitest::Assertions#assert_mock`

```ruby
module Minitest
  module Assertions
    def assert_mock(mock, msg = nil)
    end
  end
end
```

## `Minitest::Mock`

A stand-in object that answers the calls a test expects of it.

```ruby
module Minitest
  class Mock
  end
end
```

## `Minitest::Mock#expect`

Expects one call to `name`, answered with `retval`, its arguments matched by `args` or checked by the block given instead.

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

What a mock raises when what it expected is not what it received.

```ruby
class MockExpectationError
end
```

## `Object#stub`

Replaces the receiver's method `name` for the length of the block; a block given to the replaced method is called with `block_args`.

```ruby
class Object
  def stub(name, val_or_callable, *block_args, &block)
  end
end
```

## `Minitest.run`

Runs every test class defined so far under the runner's `:seed`, `:include` and `:exclude` options, and answers whether the run passed, as minitest does. What the run found reaches its reporters, which plugins add to as the run starts. `Minitest.start` runs it once the test files have run.

| Attribute | Value |
| --- | --- |
| internal | yes |

```ruby
module Minitest
  def self.run(options = {})
  end
end
```

## `Minitest.start`

Starts `Minitest.run` with the runner's options in a Fiber of its own, so a test that waits pauses the whole run. It answers `nil` while a test waits, and whether the run passed once it is over. The test runner calls it as the first process frame after the test files ran begins.

| Attribute | Value |
| --- | --- |
| internal | yes |

```ruby
module Minitest
  def self.start(options)
  end
end
```

## `Minitest.resume`

Carries the run `Minitest.start` began on from where a test waits, answering as `Minitest.start` does. The test runner calls it as each process frame and each physics frame begins until the run is over, `frame` naming which, `:process` or `:physics`, so a wait counts the frames it waits for.

| Attribute | Value |
| --- | --- |
| internal | yes |

```ruby
module Minitest
  def self.resume(frame)
  end
end
```

## `Minitest::Waits`

What a test waits for, as GUT names it, each written as a call in the test's own code. A wait pauses the run until the runner resumes it, which it cannot do from inside a block a C method calls, such as `Array#sort`'s; there it raises `FiberError`. Each test begins where a process frame begins, whatever frame the test before it waited into. Every test class includes it.

```ruby
module Minitest
  module Waits
  end
end
```

## `Minitest::Waits#wait_process_frames`

Returns once `frames` process frames have begun, before any node's `_process` in the last of them.

```ruby
module Minitest
  module Waits
    def wait_process_frames(frames)
    end
  end
end
```

## `Minitest::Waits#wait_physics_frames`

Returns once `frames` physics frames have begun, before any node's `_physics_process` in the last of them.

```ruby
module Minitest
  module Waits
    def wait_physics_frames(frames)
    end
  end
end
```

## `Minitest::Waits#wait_seconds`

Returns once the physics frames begun since the call add up to `time` seconds of the delta `_physics_process` is given, counted as GUT counts it.

```ruby
module Minitest
  module Waits
    def wait_seconds(time)
    end
  end
end
```

## `Minitest::Waits#wait_until`

Calls the block as each physics frame begins, or once each `time_between` seconds when given, until it answers `true` or `max_time` seconds have passed, counted as `wait_seconds` counts them. Answers whether the block answered `true` in time; any other answer, truthy or not, keeps it waiting, as GUT's does. Without a block it raises `ArgumentError`, as there is nothing to wait on.

```ruby
module Minitest
  module Waits
    def wait_until(max_time, time_between = 0.0)
    end
  end
end
```

## `Minitest::Waits#wait_for_signal`

Waits until `signal` is emitted or `max_time` seconds have passed, as `wait_until` waits, and answers whether it was emitted in time. What it connected to the signal is disconnected before it returns, unless the signal's object was freed meanwhile and took its connections with it.

```ruby
module Minitest
  module Waits
    def wait_for_signal(signal, max_time)
    end
  end
end
```

## `Minitest::TestRoot`

A test's way into the scene, as GUT names it: each test is given a test root, and what the test adds under it or gives to `autofree` is freed after its teardown, a node the test freed itself left alone. Every test class includes it.

```ruby
module Minitest
  module TestRoot
  end
end
```

## `Minitest::TestRoot#test_root`

The test's test root, a node in the tree. It is private, as a public method whose name starts with `test_` is a test.

```ruby
module Minitest
  module TestRoot
    def test_root
    end
  end
end
```

## `Minitest::TestRoot#add_child_autofree`

Adds `node` under the test root and frees it after the test's teardown, answering `node`.

```ruby
module Minitest
  module TestRoot
    def add_child_autofree(node)
    end
  end
end
```

## `Minitest::TestRoot#autofree`

Frees `object` after the test's teardown, answering `object`.

```ruby
module Minitest
  module TestRoot
    def autofree(object)
    end
  end
end
```

## `Minitest::SignalWatcher`

The signals a test watches, as GUT names them. What it connects is disconnected before the test's teardown, unless the object was freed with its connections. Every test class includes it.

```ruby
module Minitest
  module SignalWatcher
  end
end
```

## `Minitest::SignalWatcher#watch_signals`

Connects to every signal `object` has and counts each emission until the test's teardown, answering `object`.

```ruby
module Minitest
  module SignalWatcher
    def watch_signals(object)
    end
  end
end
```

## `Minitest::SignalWatcher#assert_signal_emitted`

Fails unless the watched `object` emitted the signal named `signal_name`, and when `object` is not watched or has no such signal. As in GUT, a `Godot::Signal` stands for the object and the name, and the argument after it is then the message.

```ruby
module Minitest
  module SignalWatcher
    def assert_signal_emitted(object, signal_name = nil, msg = nil)
    end
  end
end
```

## `Minitest::LogReporter`

The reporter the extension's plugin adds to every run, writing what it reports to the log. The extension implements the method that writes to the log on this class, so the class is where the Ruby and the Rust sides meet.

| Attribute | Value |
| --- | --- |
| internal | yes |

```ruby
module Minitest
  class LogReporter
  end
end
```
