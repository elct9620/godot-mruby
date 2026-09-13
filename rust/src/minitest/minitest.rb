# The test framework a test runner installs into a realm. It follows
# minitest's design and spelling, so Ruby developers write tests the way they
# already do, within what the extension's mruby provides.
module Minitest
  # The directory the framework's files are compiled under, and the one every
  # file the extension embeds is compiled under.
  FRAMEWORK_DIRECTORY = __FILE__[0, __FILE__.rindex("/") + 1]
  EXTENSION_DIRECTORY = FRAMEWORK_DIRECTORY[0, FRAMEWORK_DIRECTORY.index("/") + 1]

  # The calls that led to a failure, as minitest filters them: the frames
  # above the framework's first, without the extension's own; failing that,
  # every frame that is not the extension's; failing that, every frame.
  def self.filter_backtrace(backtrace)
    called = []
    backtrace.each do |frame|
      break if in_directory?(frame, FRAMEWORK_DIRECTORY)

      called << frame
    end
    [called, backtrace].each do |frames|
      filtered = frames.reject { |frame| in_directory?(frame, EXTENSION_DIRECTORY) }
      return filtered unless filtered.empty?
    end
    backtrace
  end

  def self.in_directory?(frame, directory)
    frame[0, directory.size] == directory
  end

  # A failed assertion. A test raising one fails; any other exception is an
  # error.
  class Assertion < Exception
    # Where the test failed: the first frame outside the framework.
    def location
      frame = Minitest.filter_backtrace(backtrace || []).first
      frame ? frame.split(":in ").first : "unknown"
    end

    # The file and line of #location, or nil for both when it names neither.
    def file_and_line
      separator = location.rindex(":")
      separator ? [location[0, separator], location[(separator + 1)..].to_i] : [nil, nil]
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

    # The exception, then the calls that led to it outside the framework.
    def message
      frames = Minitest.filter_backtrace(backtrace || [])
      "#{@error.class}: #{@error.message}\n    #{frames.join("\n    ")}"
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

    # The runnable methods the options' :include and :exclude leave, each
    # naming a method by name alone or as Class#name.
    def self.filter_runnable_methods(options = {})
      pos = options[:include]
      neg = options[:exclude]
      runnable_methods
        .select { |name| !pos || pos == name || pos == "#{self}##{name}" }
        .reject { |name| neg && (neg == name || neg == "#{self}##{name}") }
    end

    def self.run_suite(reporter, options = {})
      filter_runnable_methods(options).each { |name| reporter.record(new(name).run) }
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

    # The test methods in the order the run's seed shuffles them. Each class
    # reseeds, so its order does not hang on the classes before it.
    def self.runnable_methods
      srand Minitest.seed
      public_instance_methods(true).map(&:to_s).select { |name| name[0, 5] == "test_" }.sort.shuffle
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

  # What a reporter answers, as minitest's reporters do: told when the run
  # starts, of each test's result, and when the run ends, then asked whether
  # the run passed.
  class AbstractReporter
    def start; end

    def record(result); end

    def report; end

    def passed?
      true
    end
  end

  # Prints a character per test as it finishes.
  class ProgressReporter < AbstractReporter
    def record(result)
      print result.result_code
    end
  end

  # Prints the details of each test that did not pass, then the counts. The
  # run passed when every test did or was skipped.
  class SummaryReporter < AbstractReporter
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

  # The reporters of one run, told everything in the order they were added;
  # the run passed when every one of them says so.
  class CompositeReporter < AbstractReporter
    attr_reader :reporters

    def initialize(*reporters)
      @reporters = reporters
    end

    def <<(reporter)
      reporters << reporter
    end

    def start
      reporters.each(&:start)
    end

    def record(result)
      reporters.each { |reporter| reporter.record(result) }
    end

    def report
      reporters.each(&:report)
    end

    def passed?
      reporters.all?(&:passed?)
    end
  end

  class << self
    # The seed the current run shuffles its tests by.
    attr_accessor :seed

    # The run's reporter while plugins start, so they can add their own.
    attr_accessor :reporter

    # The names of the plugins each run starts.
    def extensions
      @extensions ||= []
    end
  end

  # Starts every plugin: `plugin_NAME_init` for each name in extensions.
  def self.init_plugins(options)
    extensions.each do |name|
      msg = "plugin_#{name}_init"
      send(msg, options) if respond_to?(msg)
    end
  end

  # Runs every test class defined so far, in the order the :seed option
  # shuffles them or a random seed it prints, and answers whether the run
  # passed. What the run found reaches its reporters, which plugins add to.
  def self.run(options = {})
    srand
    self.seed = options[:seed] || srand % 0xFFFF
    puts "Run options: --seed #{seed}"
    puts
    srand seed
    reporter = CompositeReporter.new(SummaryReporter.new, ProgressReporter.new)
    self.reporter = reporter
    init_plugins(options)
    self.reporter = nil
    reporter.start
    Runnable.runnables.shuffle.each { |suite| suite.run_suite(reporter, options) }
    reporter.report
    reporter.passed?
  end
end
