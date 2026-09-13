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
    # Runs of godot/test/ narrowed by a filter, each with the runner's options
    # and the summary it has to end on.
    FILTERED = {
      %w[--include SetupTest#test_one_starts_from_what_setup_set] =>
        /^1 runs, \d+ assertions, 0 failures, 0 errors, 0 skips$/,
      %w[-e SkipTest#test_a_skipped_test_does_not_fail_the_run] =>
        /^\d+ runs, \d+ assertions, 0 failures, 0 errors, 0 skips$/
    }.freeze
    # Runs that have to fail, each as the runner's options and what the run
    # has to print on the way, in the order it prints it: one per way a run
    # fails, all under godot/failing/ except the directory that does not
    # exist, the filter that names no test, and the directories that are not
    # the project's test directories.
    FAILING = {
      %w[--dir res://failing/assertion] => [
        "teardown ran after a failure",
        "FailingTest#test_one_equals_two [res://failing/assertion/failing_test.rb:9]:",
        "ERROR: FailingTest#test_one_equals_two: Expected: 1",
        "(res://failing/assertion/failing_test.rb:9)"
      ],
      %w[--dir res://failing/error] => [
        "ErrorTest#test_raises_an_argument_error [res://failing/error/error_test.rb:10]:",
        "ArgumentError: not an assertion",
        "    res://failing/error/error_test.rb:10:in refuse_the_item",
        "    res://failing/error/error_test.rb:6:in test_raises_an_argument_error",
        "ERROR: ErrorTest#test_raises_an_argument_error: ArgumentError: not an assertion",
        "(res://failing/error/error_test.rb:10)"
      ],
      %w[--dir res://failing/syntax] => ["(res://failing/syntax/broken_test.rb:4)", "0 runs, 0 assertions"],
      %w[--dir res://failing/missing] => ["The test directory res://failing/missing does not exist"],
      %w[--dir res://test --include test_nothing] => ["ERROR: Nothing ran for filter: test_nothing"],
      %w[--dir res://smoke] => ["ERROR: res://smoke is not one of the project's test directories"],
      %w[--dir res://] => ["ERROR: res:// cannot be a test directory"]
    }.freeze
    # Where the test framework's files are compiled; a failing run reports the
    # tests' own frames and never these.
    FRAMEWORK_FRAME = "godot_mruby/"

    module_function

    def verify!(project)
      verify_pass!(project)
      verify_filtered!(project)
      verify_fail!(project)
    end

    # Runs the project's Ruby tests and requires a pass that ran at least one,
    # since a run that finds nothing passes too; godot/test/ holds a skipped
    # test, which must not fail it.
    # @behavior RT-001 RT-009
    def verify_pass!(project)
      output, status = run(project, "--dir", TESTS)
      return if status.success? && output[PASSED, 1].to_i.positive?

      raise "The Ruby tests under #{TESTS} did not pass:\n#{output}"
    end

    # Runs the project's Ruby tests with each filter and requires the summary
    # it has to end on.
    # @behavior RT-024 RT-025
    def verify_filtered!(project)
      FILTERED.each do |options, summary|
        output, status = run(project, "--dir", TESTS, *options)
        next if status.success? && output.match?(summary)

        raise "The Ruby tests under #{TESTS} with #{options.join(" ")} did not end as they should:\n#{output}"
      end
    end

    # Makes each run that has to fail and requires it to fail with what it has
    # to print, in order, and without the framework's own frames.
    # @behavior RT-002 RT-003 RT-004 RT-005 RT-006 RT-007 RT-008 RT-020 RT-021 RT-022 RT-023 RT-026 RT-029 RT-030
    def verify_fail!(project)
      FAILING.each do |options, expected|
        output, status = run(project, *options)
        missing = missing_in_order(output, expected)
        missing << "no #{FRAMEWORK_FRAME} frame" if output.include?(FRAMEWORK_FRAME)
        next if status.exitstatus == 1 && missing.empty?

        raise "The Ruby tests with #{options.join(" ")} did not fail as they should #{missing}:\n#{output}"
      end
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
