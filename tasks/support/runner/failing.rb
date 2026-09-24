# frozen_string_literal: true

require_relative "../runner"

module Godot
  module Runner
    # Makes the runs of the integration-test project's Ruby tests that have to
    # fail, and reads what each reports on the way. Part of Godot.verify!.
    module Failing
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
        %W[--dir #{RUNNER_TESTS}/failing/waited] => [
          "ERROR: WaitedFailureTest#test_fails_after_a_frame: Expected: 1",
          "(#{RUNNER_TESTS}/failing/waited/waited_failure_test.rb:6)"
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

      # Makes each run that has to fail and requires it to fail with what it has
      # to print, in order, and without the framework's own frames.
      # @behavior RT-002 RT-003 RT-004 RT-005 RT-006 RT-007 RT-008 RT-010
      # @behavior RT-020 RT-021 RT-022 RT-023 RT-026 RT-029 RT-030 RT-034
      def verify!(project)
        FAILING.each do |options, expected|
          output, status = Runner.run(project, *options)
          missing = Runner.missing_in_order(output, expected)
          missing << "no #{FRAMEWORK_FRAME} frame" if output.include?(FRAMEWORK_FRAME)
          next if status.exitstatus == 1 && missing.empty?

          raise "The Ruby tests with #{options.join(" ")} did not fail as they should #{missing}:\n#{output}"
        end
      end
    end
  end
end
