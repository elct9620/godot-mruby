# The test framework the test runner prepares the game's interpreter with. It
# follows minitest's design and spelling, so Ruby developers write tests the
# way they already do, within what mruby core and mruby-metaprog provide.
module Minitest
  # A failed assertion. A test raising one fails; any other exception is an
  # error.
  class Assertion < Exception
    # Where the test failed: the first frame outside this file.
    def location
      frame = (backtrace || []).find { |line| line[0, __FILE__.size] != __FILE__ }
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

  # The assertions a test calls, with the expected value first.
  module Assertions
    def assert(test, msg = nil)
      self.assertions += 1
      return true if test

      raise Assertion, msg || "Expected #{test.inspect} to be truthy."
    end

    def refute(test, msg = nil)
      assert !test, msg || "Expected #{test.inspect} to not be truthy."
    end

    def assert_equal(exp, act, msg = nil)
      assert exp == act, msg || "Expected: #{exp.inspect}\n  Actual: #{act.inspect}"
    end

    def refute_equal(exp, act, msg = nil)
      assert exp != act, msg || "Expected #{act.inspect} to not be equal to #{exp.inspect}."
    end

    def assert_nil(obj, msg = nil)
      assert obj.nil?, msg || "Expected #{obj.inspect} to be nil."
    end

    def refute_nil(obj, msg = nil)
      assert !obj.nil?, msg || "Expected #{obj.inspect} to not be nil."
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
  # `test_` is a test, run on its own instance between `setup` and
  # `teardown`.
  class Test < Runnable
    include Assertions

    def self.runnable_methods
      public_instance_methods(true).map(&:to_s).select { |name| name[0, 5] == "test_" }.sort
    end

    def setup; end

    def teardown; end

    def run
      capture_exceptions do
        setup
        __send__(name)
      end
      capture_exceptions { teardown }
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
