# Minitest

The Ruby a test is written against, spelled the way minitest spells it so a Ruby developer writes tests as they already do. It exists only in the interpreter the test runner prepares.

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

## `Minitest::Test#setup`

Runs before each test.

```ruby
module Minitest
  class Test
    def setup
    end
  end
end
```

## `Minitest::Test#teardown`

Runs after each test, whether it passed or not.

```ruby
module Minitest
  class Test
    def teardown
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

## `Minitest.run`

Runs every test class defined so far and answers whether all of them passed. The test runner calls it once the test files have run.

| Attribute | Value |
| --- | --- |
| internal | yes |

```ruby
module Minitest
  def self.run
  end
end
```
