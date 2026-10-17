# frozen_string_literal: true

module Godot
  # Opens a copy of the integration-test project in the editor with a probe
  # plugin that runs one test directory from the test panel, and reads what
  # the panel listed and opened. The copy plays its scenes headless, so the
  # runner runs where the check does. Part of Godot.verify!.
  module Panel
    PROBE = File.join("integration", "panel", "probe")
    # Frames enough for the panel to play the runner scene and look after it.
    FRAMES = "900"
    FAILURE_TEST = "res://integration/runner/failures/assertion/assertion_test.rb"
    LIST_LINES = [
      "panel status: 1 runs, 1 failures, 0 errors, 0 skips",
      "panel row: failure | AssertionTest#test_one_equals_two | Expected: 1"
    ].freeze
    OPEN_LINE = "panel opened: #{FAILURE_TEST}:9 shown: true".freeze
    RUN_FILE_LINES = [
      "panel run file written while playing",
      "panel run file left: false"
    ].freeze

    module_function

    def verify!(project)
      output = run_probe(project)
      verify_listed!(output)
      verify_opened!(output)
      verify_run_file_removed!(output)
    end

    # @behavior RP-001
    def verify_listed!(output)
      missing = LIST_LINES.reject { |line| output.include?(line) }
      return if missing.empty?

      raise "The test panel did not list #{missing}:\n#{output}"
    end

    # @behavior RP-002
    def verify_opened!(output)
      return if output.include?(OPEN_LINE)

      raise "The test panel did not open #{OPEN_LINE}:\n#{output}"
    end

    # @behavior RP-004
    def verify_run_file_removed!(output)
      missing = RUN_FILE_LINES.reject { |line| output.include?(line) }
      return if missing.empty?

      raise "The test panel did not remove a run file left while it played, missing #{missing}:\n#{output}"
    end

    # Opens the editor on a copy of the project with the probe enabled.
    def run_probe(project)
      Godot.copy_probe_project(project, PROBE) { |copy| Godot.run_editor_frames(copy, FRAMES).first }
    end
  end
end
