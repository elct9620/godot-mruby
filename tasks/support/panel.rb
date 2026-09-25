# frozen_string_literal: true

require "fileutils"
require "tmpdir"

module Godot
  # Opens a copy of the integration-test project in the editor with a probe
  # plugin that runs one test directory from the test panel, and reads what
  # the panel listed and opened. The copy plays its scenes headless, so the
  # runner runs where the check does. Part of Godot.verify!.
  module Panel
    PROBE = File.join("integration", "panel", "probe")
    PLUGIN_SETTINGS = <<~SETTINGS
      [editor_plugins]

      enabled=PackedStringArray("res://addons/panel_probe/plugin.cfg")

      [editor]

      run/main_run_args="--headless"
    SETTINGS
    # Frames enough for the panel to play the runner scene and look after it.
    FRAMES = "900"
    FAILING = "res://integration/runner/failing/assertion/failed_assertion_test.rb"
    LISTED = [
      "panel status: 1 runs, 1 failures, 0 errors, 0 skips",
      "panel row: failure | FailedAssertionTest#test_one_equals_two | Expected: 1"
    ].freeze
    OPENED = "panel opened: #{FAILING}:9".freeze

    module_function

    def verify!(project)
      output = run_probe(project)
      verify_listed!(output)
      verify_opened!(output)
    end

    # @behavior RP-001
    def verify_listed!(output)
      missing = LISTED.reject { |line| output.include?(line) }
      return if missing.empty?

      raise "The test panel did not list #{missing}:\n#{output}"
    end

    # @behavior RP-002
    def verify_opened!(output)
      return if output.include?(OPENED)

      raise "The test panel did not open #{OPENED}:\n#{output}"
    end

    # Opens the editor on a copy of the project with the probe enabled.
    def run_probe(project)
      Dir.mktmpdir do |dir|
        copy = File.join(dir, "project")
        FileUtils.cp_r(project, copy)
        FileUtils.rm_rf(File.join(copy, ".godot", "editor"))
        FileUtils.cp_r(File.join(project, PROBE), File.join(copy, "addons", "panel_probe"))
        File.write(File.join(copy, "project.godot"), "\n#{PLUGIN_SETTINGS}", mode: "a")
        output, = Open3.capture2e(EXECUTABLE, "--headless", "--editor", "--quit-after", FRAMES, "--path", copy)
        output
      end
    end
  end
end
