# frozen_string_literal: true

require "open3"

require_relative "announcement"
require_relative "loader"
require_relative "callbacks"
require_relative "editor"
require_relative "exported_game"
require_relative "exports"
require_relative "header"
require_relative "node_scripts"
require_relative "ruby_tests"
require_relative "test_settings"

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
  PRINTS_SCENE = "res://verify/script/prints.tscn"
  # What the prints scene's Ruby prints. Each line must appear exactly once:
  # the scene attaches the same file to two nodes, and a file runs once.
  PRINTED_LINES = ["puts from mruby", "print from mruby", ":p_from_mruby"].freeze
  # A scene that gives Godot another source for a Ruby file before a node runs
  # it, and the line only that source prints.
  SOURCE_SCENE = "res://verify/script/held.tscn"
  HELD_LINE = "held.rb as Godot holds it"
  REPORT_SCENE = "res://verify/report/report.tscn"
  # What mruby says about the report scene's files, each as the line Godot
  # prints it on and the Ruby location Godot puts on the line after.
  REPORTS = {
    "WARNING: else without rescue is useless" => "(res://verify/report/warning.rb:9)",
    %(SCRIPT ERROR: syntax error, unexpected "'end'", expecting end of file) =>
      "(res://verify/report/syntax_error.rb:12)",
    "SCRIPT ERROR: wrong key" => "unlock (res://verify/report/raising.rb:10)"
  }.freeze
  # What the report scene's Ruby reports through Godot's own functions.
  PUSHED = ["ERROR: push_error from mruby", "WARNING: push_warning from mruby"].freeze
  # The backtrace Godot prints after the report scene's exception: the frames
  # Ruby called through, and nothing after them.
  BACKTRACE = [
    "Ruby backtrace (most recent call first):",
    "[0] unlock (res://verify/report/raising.rb:10)",
    "[1] _ready (res://verify/report/raising.rb:6)"
  ].freeze
  # The checks kept in modules of their own.
  CHECKS = [NodeScripts, Editor, Announcement, RubyTests, Loader, TestSettings, ExportedGame].freeze

  module_function

  # Opens the project in the editor's GUI, for a person to look at what no
  # headless run shows.
  def open_editor!(project = PROJECT)
    system(EXECUTABLE, "--editor", *editor_workarounds, "--path", project, exception: true)
  end

  # nixpkgs' macOS build aborts compiling Metal shaders for the Forward+
  # renderer (NixOS/nixpkgs#485083); the Compatibility renderer opens.
  def editor_workarounds
    version, = Open3.capture2e(EXECUTABLE, "--version")
    return [] unless RUBY_PLATFORM.include?("darwin") && version.include?(".nixpkgs.")

    %w[--rendering-method gl_compatibility]
  end

  def verify!(project = PROJECT)
    verify_loaded!(project)
    verify_scripts_run!(project)
    verify_source_held!(project)
    verify_reports!(project)
    CHECKS.each { |check| check.verify!(project) }
  end

  # The project keeps its extension list, so the editor loads the addon at
  # startup instead of discovering it: an editor that discovers an extension
  # with classes and quits at once crashes (godotengine/godot#111048). Godot
  # reports a library it could not open on stdout and may still exit 0, so the
  # pass is judged by what it printed.
  # @behavior RA-001
  def verify_loaded!(project = PROJECT)
    output, status = run_editor(project)
    errors = output.lines.grep(FAILED)
    return if status.success? && errors.empty? && output.include?(LOADED)

    raise "The extension did not load:\n#{errors.empty? ? output : errors.join}"
  end

  # Runs the prints scene for a few frames and counts the lines its Ruby
  # printed; --quit-after ends the run even when nothing does.
  # @behavior RS-001 RS-002 RS-003 RS-004
  def verify_scripts_run!(project = PROJECT)
    output, status = run_scene(project, PRINTS_SCENE, "--quit-after", "3")
    lines = output.lines.map(&:chomp)
    counts = PRINTED_LINES.to_h { |line| [line, lines.count(line)] }
    return if status.success? && output.lines.grep(FAILED).empty? && counts.values.all?(1)

    raise "The prints scene's Ruby did not print each line once #{counts}:\n#{output}"
  end

  # Runs the scene that changes a script's source in Godot, whose Ruby has to
  # be the changed source rather than the file's.
  # @behavior RS-005
  def verify_source_held!(project = PROJECT)
    output, status = run_scene(project, SOURCE_SCENE, "--quit-after", "3")
    return if status.success? && output.lines.grep(FAILED).empty? && output.lines.map(&:chomp).include?(HELD_LINE)

    raise "The script did not run the source Godot holds for it:\n#{output}"
  end

  # Runs the report scene and looks for each report followed by its location,
  # and for the exception's backtrace.
  # @behavior RR-001 RR-002 RR-003 RR-004
  def verify_reports!(project = PROJECT)
    output, status = run_scene(project, REPORT_SCENE, "--quit-after", "3")
    lines = output.lines.map(&:strip)
    missing = REPORTS.reject { |report, location| reported?(lines, report, location) }
    missing.merge!(unpushed(lines))
    missing[:backtrace] = BACKTRACE unless backtraced?(lines)
    return if status.success? && missing.empty?

    raise "The report scene's Ruby was not reported where it was written #{missing}:\n#{output}"
  end

  # What the report scene pushed that the log does not carry.
  # @behavior RC-006
  def unpushed(lines)
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

  # Opens the project in the editor, which scans it, and quits.
  def run_editor(project)
    Open3.capture2e(EXECUTABLE, "--headless", "--editor", "--quit", "--path", project)
  end

  def run_scene(project, scene, *)
    Open3.capture2e(EXECUTABLE, "--headless", "--path", project, scene, *)
  end
end
