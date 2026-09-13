# Test doubles for the test framework, following minitest-mock's design and
# spelling: a mock that answers the calls a test expects of it, and a stub
# that replaces a method for the length of a block.

# What a mock raises for a call it did not expect, or an expected call it
# never received.
class MockExpectationError < StandardError; end

module Minitest
  # A stand-in object: `expect` the calls it should receive, then `verify`
  # that it received them. Any other call raises NoMethodError.
  class Mock
    alias __respond_to? respond_to?

    # The Object methods a mock keeps, so it can still be printed and told
    # apart; a mock cannot expect them. Every other public method is removed,
    # so a call to it reaches the mock's expectations.
    KEPT_METHODS = %i[=== class inspect instance_eval instance_variables object_id public_send send to_s].freeze

    instance_methods.each do |name|
      undef_method name unless KEPT_METHODS.include?(name) || name.to_s[0, 2] == "__"
    end

    def initialize
      @expected_calls = Hash.new { |calls, name| calls[name] = [] }
      @actual_calls = Hash.new { |calls, name| calls[name] = [] }
    end

    # Expects one call to `name`, answered with `retval`. Each argument is
    # matched with `===` or `==`; a block given instead of arguments is
    # called with them and must answer truthy.
    def expect(name, retval, args = [], &blk)
      name = name.to_sym
      if blk
        raise ArgumentError, "args ignored when block given" unless args.empty?

        @expected_calls[name] << { retval: retval, block: blk }
      else
        raise ArgumentError, "args must be an array" unless args.is_a?(Array)

        @expected_calls[name] << { retval: retval, args: args }
      end
      self
    end

    def verify
      @expected_calls.each do |name, expected|
        actual = @actual_calls.key?(name) ? @actual_calls[name] : nil
        raise MockExpectationError, "Expected #{__call name, expected[0]}" unless actual
        next unless actual.size < expected.size

        raise MockExpectationError, "Expected #{__call name, expected[actual.size]}, got [#{__call name, actual}]"
      end
      true
    end

    def respond_to?(name, include_private = false)
      return true if @expected_calls.key?(name.to_sym)

      __respond_to?(name, include_private)
    end

    def method_missing(name, *args, &block)
      unless @expected_calls.key?(name)
        raise NoMethodError, format("unmocked method %p, expected one of %p", name, @expected_calls.keys.sort { |a, b| a.to_s <=> b.to_s })
      end

      expected_call = @expected_calls[name][@actual_calls[name].size]
      raise MockExpectationError, format("No more expects available for %p: %p", name, args) unless expected_call

      if expected_call[:block]
        @actual_calls[name] << expected_call
        raise MockExpectationError, format("mocked method %p failed block w/ %p", name, args) unless expected_call[:block].call(*args, &block)

        return expected_call[:retval]
      end

      expected_args = expected_call[:args]
      if expected_args.size != args.size
        raise ArgumentError, format("mocked method %p expects %d arguments, got %p", name, expected_args.size, args)
      end

      unless arguments_match?(expected_args, args)
        raise MockExpectationError, format("mocked method %p called with unexpected arguments %p", name, args)
      end

      @actual_calls[name] << { retval: expected_call[:retval], args: args }
      expected_call[:retval]
    end

    private

    def arguments_match?(expected_args, args)
      matched = true
      expected_args.each_with_index { |expected, index| matched &&= expected === args[index] || expected == args[index] }
      matched
    end

    def __call(name, data)
      return data.map { |call| __call(name, call) }.join(", ") if data.is_a?(Array)

      "#{name}(#{data[:args].inspect[1..-2]}) => #{data[:retval].inspect}"
    end
  end

  module Assertions
    # Passes when the mock received every call it expected.
    def assert_mock(mock, msg = nil)
      assert mock.verify
    rescue MockExpectationError => e
      msg = message(msg) { e.message }
      flunk msg
    end
  end
end

class Object
  # Replaces this object's method `name` for the length of the block, then
  # restores it. A callable replacement is called with the arguments;
  # anything else is returned, calling the caller's block with `block_args`
  # first.
  def stub(name, val_or_callable, *block_args, &block)
    new_name = "__minitest_stub__#{name}"
    metaclass = singleton_class
    metaclass.__send__(:alias_method, new_name, name)
    metaclass.__send__(:define_method, name) do |*args, &blk|
      if val_or_callable.respond_to?(:call)
        val_or_callable.call(*args, &blk)
      else
        blk&.call(*block_args)
        val_or_callable
      end
    end

    block.call(self)
  ensure
    metaclass.__send__(:undef_method, name)
    metaclass.__send__(:alias_method, name, new_name)
    metaclass.__send__(:undef_method, new_name)
  end
end
