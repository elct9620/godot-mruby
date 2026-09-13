# frozen_string_literal: true

require "open3"

require_relative "ruby_tests"

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

  module_function

  def verify!(project = PROJECT)
    verify_loaded!(project)
    verify_scripts_run!(project)
    verify_reports!(project)
    RubyTests.verify!(project)
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

  def run_scene(project, scene, *)
    Open3.capture2e(EXECUTABLE, "--headless", "--path", project, scene, *)
  end
end
