# frozen_string_literal: true

require "open3"

# Runs the integration-test project headless and reads what it reports.
# Backs tasks/godot.rake.
module Godot
  PROJECT = File.expand_path("../../godot", __dir__)
  # The one file of the project's .godot/ the repository keeps.
  EXTENSION_LIST = File.join(".godot", "extension_list.cfg")
  EXECUTABLE = ENV.fetch("GODOT", "godot")
  # What gdext prints once the engine has called into the library.
  LOADED = "Initialize godot-rust"
  FAILED = /^(ERROR|SCRIPT ERROR):/
  SMOKE_SCENE = "res://smoke/hello.tscn"
  # What the smoke scene's Ruby prints. Each line must appear exactly once:
  # the scene attaches the same file to two nodes, and a file runs once.
  SMOKE_LINES = ["puts from mruby", "print from mruby", ":p_from_mruby"].freeze
  REPORT_SCENE = "res://report/report.tscn"
  # What mruby says about the report scene's files, each as the line Godot
  # prints it on and the Ruby location Godot puts on the line after.
  REPORTS = {
    "WARNING: else without rescue is useless" => "(res://report/warning.rb:6)",
    %(SCRIPT ERROR: syntax error, unexpected "'end'", expecting end of file) => "(res://report/syntax_error.rb:3)"
  }.freeze
  RUNNER_SCENE = "res://addons/godot_mruby/runner.tscn"
  # The runner answers within the frame it starts in; this only stops a
  # runner that never quits from hanging the check.
  RUNNER_FRAMES = "60"
  TESTS = "res://test"
  PASSED = /^(\d+) runs, \d+ assertions, 0 failures, 0 errors, \d+ skips$/
  # Test directories whose run has to fail, each with what the run has to
  # print on the way: one per way a run fails, all under godot/failing/
  # except the one that does not exist.
  FAILING = {
    "res://failing/assertion" => [
      "FailingTest#test_one_equals_two [res://failing/assertion/failing_test.rb:9]:",
      "teardown ran after a failure"
    ],
    "res://failing/error" => [
      "ErrorTest#test_raises_an_argument_error [res://failing/error/error_test.rb:10]:",
      "ArgumentError: not an assertion",
      "    res://failing/error/error_test.rb:10:in refuse_the_item",
      "    res://failing/error/error_test.rb:6:in test_raises_an_argument_error"
    ],
    "res://failing/syntax" => ["(res://failing/syntax/broken_test.rb:4)"],
    "res://failing/missing" => ["The test directory res://failing/missing does not exist"]
  }.freeze
  # Where the test framework's files are compiled; a failing run reports the
  # tests' own frames and never these.
  FRAMEWORK_FRAME = "godot_mruby/"

  module_function

  def verify!(project = PROJECT)
    verify_loaded!(project)
    verify_scripts_run!(project)
    verify_reports!(project)
    verify_tests_pass!(project)
    verify_tests_fail!(project)
  end

  # The project keeps its extension list, so the editor loads the addon at
  # startup instead of discovering it: an editor that discovers an extension
  # with classes and quits at once crashes (godotengine/godot#111048). Godot
  # reports a library it could not open on stdout and may still exit 0, so the
  # pass is judged by what it printed.
  def verify_loaded!(project = PROJECT)
    output, status = Open3.capture2e(EXECUTABLE, "--headless", "--editor", "--quit", "--path", project)
    errors = output.lines.grep(FAILED)
    return if status.success? && errors.empty? && output.include?(LOADED)

    raise "The extension did not load:\n#{errors.empty? ? output : errors.join}"
  end

  # Runs the smoke scene for a few frames and counts the lines its Ruby
  # printed; --quit-after ends the run even when nothing does.
  # @behavior RS-001 RS-002 RS-003 RS-004
  def verify_scripts_run!(project = PROJECT)
    output, status = run_scene(project, SMOKE_SCENE, "--quit-after", "3")
    lines = output.lines.map(&:chomp)
    counts = SMOKE_LINES.to_h { |line| [line, lines.count(line)] }
    return if status.success? && output.lines.grep(FAILED).empty? && counts.values.all?(1)

    raise "The smoke scene's Ruby did not print each line once #{counts}:\n#{output}"
  end

  # Runs the report scene and looks for each report followed by its location.
  # @behavior RR-001 RR-002
  def verify_reports!(project = PROJECT)
    output, status = run_scene(project, REPORT_SCENE, "--quit-after", "3")
    lines = output.lines.map(&:strip)
    missing = REPORTS.reject { |report, location| reported?(lines, report, location) }
    return if status.success? && missing.empty?

    raise "The report scene's Ruby was not reported where it was written #{missing}:\n#{output}"
  end

  def reported?(lines, report, location)
    lines.each_cons(2).any? { |line, at| line == report && at.start_with?("at:") && at.end_with?(location) }
  end

  # Runs the project's Ruby tests and requires a pass that ran at least one,
  # since a run that finds nothing passes too; godot/test/ holds a skipped
  # test, which must not fail it.
  # @behavior RT-001 RT-009
  def verify_tests_pass!(project = PROJECT)
    output, status = run_tests(project, TESTS)
    return if status.success? && output[PASSED, 1].to_i.positive?

    raise "The Ruby tests under #{TESTS} did not pass:\n#{output}"
  end

  # Runs each directory under godot/failing/ and requires the run to fail
  # with what it has to print, and without the framework's own frames.
  # @behavior RT-002 RT-003 RT-004 RT-005 RT-006 RT-007 RT-008 RT-020
  def verify_tests_fail!(project = PROJECT)
    FAILING.each do |dir, expected|
      output, status = run_tests(project, dir)
      missing = expected.reject { |line| output.include?(line) }
      missing << "no #{FRAMEWORK_FRAME} frame" if output.include?(FRAMEWORK_FRAME)
      next if status.exitstatus == 1 && missing.empty?

      raise "The Ruby tests under #{dir} did not fail as they should #{missing}:\n#{output}"
    end
  end

  def run_tests(project, dir)
    run_scene(project, RUNNER_SCENE, "--quit-after", RUNNER_FRAMES, "--", "--dir", dir)
  end

  def run_scene(project, scene, *)
    Open3.capture2e(EXECUTABLE, "--headless", "--path", project, scene, *)
  end
end
