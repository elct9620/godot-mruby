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
      "WARNING: else without rescue is useless" => "(res://integration/report/warner.rb:9)",
      %(SCRIPT ERROR: syntax error, unexpected "'end'", expecting end of file) =>
        "(res://integration/report/typo.rb:12)",
      "SCRIPT ERROR: wrong key" => "unlock (res://integration/report/fuse.rb:10)"
    }.freeze
    # What the report scene's Ruby reports through Godot's own functions.
    PUSH_LINES = ["ERROR: push_error from mruby", "WARNING: push_warning from mruby"].freeze
    # The backtraces Godot prints after the report scene's exception and what
    # its Ruby pushes: the frames Ruby called through, and nothing after them.
    BACKTRACES = {
      exception: ["Ruby backtrace (most recent call first):",
                  "[0] unlock (res://integration/report/fuse.rb:10)",
                  "[1] _ready (res://integration/report/fuse.rb:6)"],
      error: ["Ruby backtrace (most recent call first):",
              "[0] proclaim (res://integration/report/herald.rb:11)",
              "[1] _ready (res://integration/report/herald.rb:7)"],
      warning: ["Ruby backtrace (most recent call first):",
                "[0] proclaim (res://integration/report/herald.rb:12)",
                "[1] _ready (res://integration/report/herald.rb:7)"]
    }.freeze

    module_function

    # Runs the report scene and looks for each report followed by its location,
    # and for the backtraces of the exception and of what Ruby pushed.
    # @behavior RR-001 RR-002 RR-003 RR-004 RR-006 RR-009 RR-010
    def verify!(project)
      output, status = Godot.run_scene(project, SCENE, "--quit-after", "3")
      lines = output.lines.map(&:strip)
      missing = REPORTS.reject { |report, location| reported?(lines, report, location) }
      missing.merge!(find_missing_pushes(lines))
      missing.merge!(BACKTRACES.reject { |_, backtrace| backtraced?(lines, backtrace) })
      return if status.success? && missing.empty?

      raise "The report scene's Ruby was not reported where it was written #{missing}:\n#{output}"
    end

    # What the report scene pushed that the log does not carry.
    # @behavior RC-006
    def find_missing_pushes(lines)
      PUSH_LINES.reject { |line| lines.include?(line) }.to_h { |line| [line, "anywhere"] }
    end

    def reported?(lines, report, location)
      lines.each_cons(2).any? { |line, at| line == report && at.start_with?("at:") && at.end_with?(location) }
    end

    def backtraced?(lines, backtrace)
      [*lines, ""].each_cons(backtrace.size + 1).any? do |*frames, after|
        frames == backtrace && !after.start_with?("[")
      end
    end
  end
end
