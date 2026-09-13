# frozen_string_literal: true

require "open3"

# Runs the integration-test project headless and reads what it reports.
# Backs tasks/godot.rake.
module Godot
  PROJECT = File.expand_path("../../godot", __dir__)
  EXECUTABLE = ENV.fetch("GODOT", "godot")
  # What gdext prints once the engine has called into the library.
  LOADED = "Initialize godot-rust"
  FAILED = /^(ERROR|SCRIPT ERROR):/
  SMOKE_SCENE = "res://smoke/hello.tscn"
  # What the smoke scene's Ruby prints. Each line must appear exactly once:
  # the scene attaches the same file to two nodes, and a file runs once.
  SMOKE_LINES = ["puts from mruby", "print from mruby", ":p_from_mruby"].freeze

  module_function

  # The editor pass comes first, since it records the extension a plain run
  # then loads.
  def verify!(project = PROJECT)
    verify_loaded!(project)
    verify_scripts_run!(project)
  end

  # An editor pass is what records the addon's .gdextension for later runs and
  # loads it; the project has no main scene for a plain run to start. Godot
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
    output, status = Open3.capture2e(EXECUTABLE, "--headless", "--path", project, SMOKE_SCENE, "--quit-after", "3")
    lines = output.lines.map(&:chomp)
    counts = SMOKE_LINES.to_h { |line| [line, lines.count(line)] }
    return if status.success? && output.lines.grep(FAILED).empty? && counts.values.all?(1)

    raise "The smoke scene's Ruby did not print each line once #{counts}:\n#{output}"
  end
end
