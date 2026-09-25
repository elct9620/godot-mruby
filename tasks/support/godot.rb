# frozen_string_literal: true

require "open3"

require_relative "declarations"
require_relative "export"
require_relative "loader"
require_relative "report"
require_relative "runner"
require_relative "runner/failing"
require_relative "runner/results"
require_relative "runner/settings"
require_relative "script"

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
  # The checks, each in the module named after the behaviour it claims.
  CHECKS = [
    Script, Declarations, Report, Runner, Runner::Failing, Runner::Results, Runner::Settings, Loader, Export
  ].freeze

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

  # Opens the project in the editor, which scans it, and quits.
  def run_editor(project)
    Open3.capture2e(EXECUTABLE, "--headless", "--editor", "--quit", "--path", project)
  end

  def run_scene(project, scene, *)
    Open3.capture2e(EXECUTABLE, "--headless", "--path", project, scene, *)
  end
end
