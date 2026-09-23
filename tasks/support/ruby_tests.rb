# frozen_string_literal: true

module Godot
  # Runs the integration-test project's Ruby tests through the addon's runner
  # scene and reads what the runs report. Part of Godot.verify!.
  module RubyTests
    RUNNER_SCENE = "res://addons/godot_mruby/runner.tscn"
    # The runner answers within the frame it starts in; this only stops a
    # runner that never quits from hanging the check.
    RUNNER_FRAMES = "60"
    TESTS = "res://test"
    PASSED = /^(\d+) runs, \d+ assertions, 0 failures, 0 errors, \d+ skips$/
    RUNNER_TESTS = "res://integration/runner"
    # Runs that have to pass, each with the runner's options and the summary it
    # has to end on: a directory whose one test passes and the other skips,
    # and runs narrowed by a filter.
    PASSING = {
      %W[--dir #{RUNNER_TESTS}/skip] => /^2 runs, \d+ assertions, 0 failures, 0 errors, 1 skips$/,
      %W[--dir #{RUNNER_TESTS}/order --include AlphaOrderTest#test_a] =>
        /^1 runs, \d+ assertions, 0 failures, 0 errors, 0 skips$/,
      %W[--dir #{RUNNER_TESTS}/skip --exclude SkipTest#test_skips] =>
        /^1 runs, \d+ assertions, 0 failures, 0 errors, 0 skips$/
    }.freeze
    # A test directory whose tests print their names as they run, the seeds
    # tried on it, and how a run prints its seed and each test it ran.
    ORDER = "#{RUNNER_TESTS}/order".freeze
    SEEDS = %w[1 2 3 4 5].freeze
    SEED = /^Run options: --seed (\d+)$/
    RAN = /ran (\w+#test_\w+)/
    # Runs that have to fail, each as the runner's options and what the run
    # has to print on the way, in the order it prints it: one per way a run
    # fails, all under godot/integration/runner/failing/ except the filter that
    # names no test, the directories that are not the project's test
    # directories, and a seed that is not a number. An error's calls end at the
    # test method, so its frame is the last before the report's blank line.
    FAILING = {
      %W[--dir #{RUNNER_TESTS}/failing/assertion] => [
        "teardown ran after a failure",
        "FailedAssertionTest#test_one_equals_two [#{RUNNER_TESTS}/failing/assertion/failed_assertion_test.rb:9]:",
        "ERROR: FailedAssertionTest#test_one_equals_two: Expected: 1",
        "(#{RUNNER_TESTS}/failing/assertion/failed_assertion_test.rb:9)"
      ],
      %W[--dir #{RUNNER_TESTS}/failing/error] => [
        "RaisedErrorTest#test_raises_an_argument_error [#{RUNNER_TESTS}/failing/error/raised_error_test.rb:10]:",
        "ArgumentError: not an assertion",
        "    #{RUNNER_TESTS}/failing/error/raised_error_test.rb:10:in refuse_the_item",
        "    #{RUNNER_TESTS}/failing/error/raised_error_test.rb:6:in test_raises_an_argument_error\n\n",
        "ERROR: RaisedErrorTest#test_raises_an_argument_error: ArgumentError: not an assertion",
        "(#{RUNNER_TESTS}/failing/error/raised_error_test.rb:10)"
      ],
      %W[--dir #{RUNNER_TESTS}/failing/syntax] => [
        "(#{RUNNER_TESTS}/failing/syntax/broken_syntax_test.rb:4)",
        "0 runs, 0 assertions"
      ],
      %W[--dir #{RUNNER_TESTS}/failing/missing] => [
        "The test directory #{RUNNER_TESTS}/failing/missing does not exist"
      ],
      %w[--dir res://test --include test_nothing] => ["ERROR: Nothing ran for filter: test_nothing"],
      %w[--dir res://src] => ["ERROR: res://src is not one of the project's test directories"],
      %w[--dir res://] => ["ERROR: res:// cannot be a test directory"],
      %w[--dir res://test --seed x] => ["ERROR: --seed takes a whole number, not x"]
    }.freeze
    # Where the test framework's files are compiled; a failing run reports the
    # tests' own frames and never these.
    FRAMEWORK_FRAME = "godot_mruby/"

    module_function

    def verify!(project)
      verify_pass!(project)
      verify_passing!(project)
      verify_fail!(project)
      verify_seed!(project)
      verify_shuffled!(project)
    end

    # Runs the project's Ruby tests and requires a pass that ran at least one,
    # since a run that finds nothing passes too.
    # @behavior RT-001
    def verify_pass!(project)
      output, status = run(project, "--dir", TESTS)
      return if status.success? && output[PASSED, 1].to_i.positive?

      raise "The Ruby tests under #{TESTS} did not pass:\n#{output}"
    end

    # Makes each run that has to pass and requires the summary it has to end
    # on.
    # @behavior RT-009 RT-024 RT-025
    def verify_passing!(project)
      PASSING.each do |options, summary|
        output, status = run(project, *options)
        next if status.success? && output.match?(summary)

        raise "The Ruby tests with #{options.join(" ")} did not end as they should:\n#{output}"
      end
    end

    # Makes each run that has to fail and requires it to fail with what it has
    # to print, in order, and without the framework's own frames.
    # @behavior RT-002 RT-003 RT-004 RT-005 RT-006 RT-007 RT-008 RT-020 RT-021 RT-022 RT-023 RT-026 RT-029 RT-030 RT-034
    def verify_fail!(project)
      FAILING.each do |options, expected|
        output, status = run(project, *options)
        missing = missing_in_order(output, expected)
        missing << "no #{FRAMEWORK_FRAME} frame" if output.include?(FRAMEWORK_FRAME)
        next if status.exitstatus == 1 && missing.empty?

        raise "The Ruby tests with #{options.join(" ")} did not fail as they should #{missing}:\n#{output}"
      end
    end

    # Runs the order directory without a seed, which has to print the one it
    # used, then again with that seed, which has to repeat the order.
    # @behavior RT-031 RT-032
    def verify_seed!(project)
      output, = run(project, "--dir", ORDER)
      seed = output[SEED, 1]
      raise "A run of #{ORDER} did not print its seed:\n#{output}" unless seed

      again, = run(project, "--dir", ORDER, "--seed", seed)
      return if order_of(output).any? && order_of(again) == order_of(output)

      raise "The seed #{seed} did not repeat the order of #{ORDER}:\n#{output}\n#{again}"
    end

    # Runs the order directory with each seed; one of them has to run its
    # tests out of name order.
    # @behavior RT-033
    def verify_shuffled!(project)
      orders = SEEDS.map { |seed| order_of(run(project, "--dir", ORDER, "--seed", seed).first) }
      return if orders.any? { |order| order.any? && order != order.sort }

      raise "No seed of #{SEEDS.join(", ")} ran #{ORDER} out of name order: #{orders}"
    end

    def order_of(output)
      output.scan(RAN).flatten
    end

    # The expected lines that do not appear in the output after the one before
    # them.
    def missing_in_order(output, expected)
      position = 0
      expected.reject do |line|
        found = output.index(line, position)
        position = found + line.size if found
        found
      end
    end

    def run(project, *options)
      Godot.run_scene(project, RUNNER_SCENE, "--quit-after", RUNNER_FRAMES, "--", *options)
    end
  end
end
