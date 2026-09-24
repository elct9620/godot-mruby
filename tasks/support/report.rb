# frozen_string_literal: true

module Godot
  # Runs the integration-test project's scene whose Ruby warns, fails to
  # parse, raises and pushes errors, and reads where each is reported. Part
  # of Godot.verify!.
  module Report
    SCENE = "res://integration/report/report.tscn"
    # What mruby says about the report scene's files, each as the line Godot
    # prints it on and the Ruby location Godot puts on the line after.
    REPORTS = {
      "WARNING: else without rescue is useless" => "(res://integration/report/warning.rb:9)",
      %(SCRIPT ERROR: syntax error, unexpected "'end'", expecting end of file) =>
        "(res://integration/report/syntax_error.rb:12)",
      "SCRIPT ERROR: wrong key" => "unlock (res://integration/report/raising.rb:10)"
    }.freeze
    # What the report scene's Ruby reports through Godot's own functions.
    PUSHED = ["ERROR: push_error from mruby", "WARNING: push_warning from mruby"].freeze
    # The backtrace Godot prints after the report scene's exception: the frames
    # Ruby called through, and nothing after them.
    BACKTRACE = [
      "Ruby backtrace (most recent call first):",
      "[0] unlock (res://integration/report/raising.rb:10)",
      "[1] _ready (res://integration/report/raising.rb:6)"
    ].freeze

    module_function

    # Runs the report scene and looks for each report followed by its location,
    # and for the exception's backtrace.
    # @behavior RR-001 RR-002 RR-003 RR-004
    def verify!(project)
      output, status = Godot.run_scene(project, SCENE, "--quit-after", "3")
      lines = output.lines.map(&:strip)
      missing = REPORTS.reject { |report, location| reported?(lines, report, location) }
      missing.merge!(missing_pushes(lines))
      missing[:backtrace] = BACKTRACE unless backtraced?(lines)
      return if status.success? && missing.empty?

      raise "The report scene's Ruby was not reported where it was written #{missing}:\n#{output}"
    end

    # What the report scene pushed that the log does not carry.
    # @behavior RC-006
    def missing_pushes(lines)
      PUSHED.reject { |line| lines.include?(line) }.to_h { |line| [line, "anywhere"] }
    end

    def reported?(lines, report, location)
      lines.each_cons(2).any? { |line, at| line == report && at.start_with?("at:") && at.end_with?(location) }
    end

    def backtraced?(lines)
      [*lines, ""].each_cons(BACKTRACE.size + 1).any? do |*frames, after|
        frames == BACKTRACE && !after.start_with?("[")
      end
    end
  end
end
