# The test framework the test runner prepares the game's interpreter with. It
# follows minitest's design and spelling, so Ruby developers write tests the
# way they already do, within what the extension's mruby provides.
module Minitest
  # A failed assertion. A test raising one fails; any other exception is an
  # error.
  # The directory the framework's files are compiled under; a frame there is
  # the framework's own rather than the test's.
  FRAMEWORK_DIRECTORY = __FILE__[0, __FILE__.rindex("/") + 1]

  class Assertion < Exception
    # Where the test failed: the first frame outside the framework.
    def location
      frame = (backtrace || []).find { |line| line[0, FRAMEWORK_DIRECTORY.size] != FRAMEWORK_DIRECTORY }
      frame ? frame.split(":in ").first : "unknown"
    end

    def result_code
      "F"
    end

    def result_label
      "Failure"
    end
  end

  # Ends a test without failing it.
  class Skip < Assertion
    def result_code
      "S"
    end

    def result_label
      "Skipped"
    end
  end

  # An exception other than an assertion, raised while a test ran.
  class UnexpectedError < Assertion
    attr_reader :error

    def initialize(error)
      super("#{error.class}: #{error.message}")
      @error = error
    end

    def backtrace
      @error.backtrace
    end

    def result_code
      "E"
    end

    def result_label
      "Error"
    end
  end

  # The assertions a test calls, with the expected value first. Each builds
  # its failure message only once it fails, and a message given to it comes
  # before its own.
  module Assertions
    # What assert_operator is given when it has no second operand.
    UNDEFINED = Object.new
  
    def UNDEFINED.inspect
      "UNDEFINED"
    end
  
    def mu_pp(obj)
      obj.inspect
    end
  
    def message(msg = nil, ending = ".", &default)
      return msg if msg.is_a?(Proc)
  
      Proc.new do
        custom_message = "#{msg}.\n" unless msg.nil? || msg.to_s.empty?
        "#{custom_message}#{default.call}#{ending}"
      end
    end
  
    def assert(test, msg = nil)
      self.assertions += 1
      return true if test
  
      msg ||= "Expected #{mu_pp test} to be truthy."
      msg = msg.call if msg.is_a?(Proc)
      raise Assertion, msg
    end
  
    def refute(test, msg = nil)
      msg ||= message { "Expected #{mu_pp test} to not be truthy" }
      assert !test, msg
    end
  
    def pass(_msg = nil)
      assert true
    end
  
    def assert_equal(exp, act, msg = nil)
      msg = message(msg, nil) { "Expected: #{mu_pp exp}\n  Actual: #{mu_pp act}" }
      refute_nil exp, message { "Use assert_nil if expecting nil" } if exp.nil?
      assert exp == act, msg
    end
  
    def refute_equal(exp, act, msg = nil)
      msg = message(msg) { "Expected #{mu_pp act} to not be equal to #{mu_pp exp}" }
      refute exp == act, msg
    end
  
    def assert_nil(obj, msg = nil)
      msg = message(msg) { "Expected #{mu_pp obj} to be nil" }
      assert obj.nil?, msg
    end
  
    def refute_nil(obj, msg = nil)
      msg = message(msg) { "Expected #{mu_pp obj} to not be nil" }
      refute obj.nil?, msg
    end
  
    def assert_empty(obj, msg = nil)
      msg = message(msg) { "Expected #{mu_pp obj} to be empty" }
      assert_predicate obj, :empty?, msg
    end
  
    def refute_empty(obj, msg = nil)
      msg = message(msg) { "Expected #{mu_pp obj} to not be empty" }
      refute_predicate obj, :empty?, msg
    end
  
    def assert_in_delta(exp, act, delta = 0.001, msg = nil)
      n = (exp - act).abs
      msg = message(msg) { "Expected |#{exp} - #{act}| (#{n}) to be <= #{delta}" }
      assert delta >= n, msg
    end
  
    def refute_in_delta(exp, act, delta = 0.001, msg = nil)
      n = (exp - act).abs
      msg = message(msg) { "Expected |#{exp} - #{act}| (#{n}) to not be <= #{delta}" }
      refute delta >= n, msg
    end
  
    def assert_in_epsilon(exp, act, epsilon = 0.001, msg = nil)
      assert_in_delta exp, act, [exp.abs, act.abs].min * epsilon, msg
    end
  
    def refute_in_epsilon(exp, act, epsilon = 0.001, msg = nil)
      refute_in_delta exp, act, [exp.abs, act.abs].min * epsilon, msg
    end
  
    def assert_includes(collection, obj, msg = nil)
      msg = message(msg) { "Expected #{mu_pp collection} to include #{mu_pp obj}" }
      assert_operator collection, :include?, obj, msg
    end
  
    def refute_includes(collection, obj, msg = nil)
      msg = message(msg) { "Expected #{mu_pp collection} to not include #{mu_pp obj}" }
      refute_operator collection, :include?, obj, msg
    end
  
    def assert_instance_of(cls, obj, msg = nil)
      msg = message(msg) { "Expected #{mu_pp obj} to be an instance of #{cls}, not #{obj.class}" }
      assert obj.instance_of?(cls), msg
    end
  
    def refute_instance_of(cls, obj, msg = nil)
      msg = message(msg) { "Expected #{mu_pp obj} to not be an instance of #{cls}" }
      refute obj.instance_of?(cls), msg
    end
  
    def assert_kind_of(cls, obj, msg = nil)
      msg = message(msg) { "Expected #{mu_pp obj} to be a kind of #{cls}, not #{obj.class}" }
      assert obj.kind_of?(cls), msg
    end
  
    def refute_kind_of(cls, obj, msg = nil)
      msg = message(msg) { "Expected #{mu_pp obj} to not be a kind of #{cls}" }
      refute obj.kind_of?(cls), msg
    end
  
    def assert_operator(o1, op, o2 = UNDEFINED, msg = nil)
      return assert_predicate o1, op, msg if UNDEFINED == o2
  
      assert_respond_to o1, op
      msg = message(msg) { "Expected #{mu_pp o1} to be #{op} #{mu_pp o2}" }
      assert o1.__send__(op, o2), msg
    end
  
    def refute_operator(o1, op, o2 = UNDEFINED, msg = nil)
      return refute_predicate o1, op, msg if UNDEFINED == o2
  
      assert_respond_to o1, op
      msg = message(msg) { "Expected #{mu_pp o1} to not be #{op} #{mu_pp o2}" }
      refute o1.__send__(op, o2), msg
    end
  
    def assert_predicate(o1, op, msg = nil)
      assert_respond_to o1, op, include_all: true
      msg = message(msg) { "Expected #{mu_pp o1} to be #{op}" }
      assert o1.__send__(op), msg
    end
  
    def refute_predicate(o1, op, msg = nil)
      assert_respond_to o1, op, include_all: true
      msg = message(msg) { "Expected #{mu_pp o1} to not be #{op}" }
      refute o1.__send__(op), msg
    end
  
    def assert_respond_to(obj, meth, msg = nil, include_all: false)
      msg = message(msg) { "Expected #{mu_pp obj} (#{obj.class}) to respond to ##{meth}" }
      assert obj.respond_to?(meth, include_all), msg
    end
  
    def refute_respond_to(obj, meth, msg = nil, include_all: false)
      msg = message(msg) { "Expected #{mu_pp obj} to not respond to #{meth}" }
      refute obj.respond_to?(meth, include_all), msg
    end
  
    def assert_same(exp, act, msg = nil)
      msg = message(msg) do
        format("Expected %s (oid=%d) to be the same as %s (oid=%d)", mu_pp(act), act.object_id, mu_pp(exp), exp.object_id)
      end
      refute_nil exp, message { "Use assert_nil if expecting nil" } if exp.nil?
      assert exp.equal?(act), msg
    end
  
    def refute_same(exp, act, msg = nil)
      msg = message(msg) do
        format("Expected %s (oid=%d) to not be the same as %s (oid=%d)", mu_pp(act), act.object_id, mu_pp(exp), exp.object_id)
      end
      refute exp.equal?(act), msg
    end
  
    def assert_raises(*exp)
      msg = exp.last.is_a?(String) ? "#{exp.pop}.\n" : ""
      exp << StandardError if exp.empty?
      begin
        yield
      rescue Exception => e
        if exp.any? { |klass| e.is_a?(klass) }
          assert true
          return e
        end
        raise if e.is_a?(Assertion)
  
        flunk "#{msg}#{exp.inspect} exception expected, not\nClass: <#{e.class}>\nMessage: <#{e.message.inspect}>"
      end
      flunk "#{msg}#{exp.inspect} expected but nothing was raised."
    end
  
    def flunk(msg = nil)
      assert false, msg || "Epic Fail!"
    end
  
    def skip(msg = nil)
      raise Skip, msg || "Skipped, no message given"
    end
  end
  
  # What can be run and reported: every subclass is collected as it is
  # defined, so a test file needs nothing but its class.
  class Runnable
    @@runnables = []

    def self.runnables
      @@runnables
    end

    def self.inherited(klass)
      @@runnables << klass
      super
    end

    def self.runnable_methods
      []
    end

    def self.run_suite(reporter)
      runnable_methods.each { |name| reporter.record(new(name).run) }
    end

    attr_accessor :assertions
    attr_reader :name, :failures

    def initialize(name)
      @name = name
      @failures = []
      @assertions = 0
    end

    def failure
      failures.first
    end

    def passed?
      failure.nil?
    end

    def skipped?
      failure.is_a?(Skip)
    end

    def result_code
      failure ? failure.result_code : "."
    end
  end

  # What a test class inherits: each public method whose name starts with
  # `test_` is a test, run on its own instance between its lifecycle hooks.
  class Test < Runnable
    SETUP_METHODS = %w[before_setup setup after_setup].freeze
    TEARDOWN_METHODS = %w[before_teardown teardown after_teardown].freeze

    # The steps around each test. A test class overrides `setup` and
    # `teardown`; a library includes a module overriding the `before_` and
    # `after_` hooks, calling `super`. They live in a module so a module
    # included into Minitest::Test itself still comes before them.
    module LifecycleHooks
      def before_setup; end

      def setup; end

      def after_setup; end

      def before_teardown; end

      def teardown; end

      def after_teardown; end
    end

    include Assertions
    include LifecycleHooks

    def self.runnable_methods
      public_instance_methods(true).map(&:to_s).select { |name| name[0, 5] == "test_" }.sort
    end

    # Each teardown step runs whatever the steps before it raised.
    def run
      capture_exceptions do
        SETUP_METHODS.each { |hook| __send__(hook) }
        __send__(name)
      end
      TEARDOWN_METHODS.each { |hook| capture_exceptions { __send__(hook) } }
      self
    end

    private

    def capture_exceptions
      yield
    rescue Assertion => e
      failures << e
    rescue Exception => e
      failures << UnexpectedError.new(e)
    end
  end

  # Prints a run the way minitest does: a character per test, the details of
  # each test that did not pass, then the counts.
  class Reporter
    def initialize
      @count = 0
      @assertions = 0
      @results = []
    end

    def start
      puts "# Running:"
      puts
    end

    def record(result)
      @count += 1
      @assertions += result.assertions
      @results << result unless result.passed?
      print result.result_code
    end

    def report
      puts
      problems.each_with_index do |result, index|
        failure = result.failure
        puts
        puts "  #{index + 1}) #{failure.result_label}:"
        puts "#{result.class}##{result.name} [#{failure.location}]:"
        puts failure.message
      end
      puts
      puts summary
    end

    def passed?
      problems.empty?
    end

    private

    def problems
      @results.reject(&:skipped?)
    end

    def summary
      errors = problems.select { |result| result.failure.is_a?(UnexpectedError) }.size
      failures = problems.size - errors
      skips = @results.size - problems.size
      "#{@count} runs, #{@assertions} assertions, #{failures} failures, #{errors} errors, #{skips} skips"
    end
  end

  # Runs every test class defined so far, in name order, and answers whether
  # all of them passed.
  def self.run
    reporter = Reporter.new
    reporter.start
    Runnable.runnables.sort { |a, b| a.to_s <=> b.to_s }.each { |suite| suite.run_suite(reporter) }
    reporter.report
    reporter.passed?
  end
end
