# frozen_string_literal: true

require "fileutils"
require "open3"
require "tmpdir"

require_relative "declarations"
require_relative "export"
require_relative "loader"
require_relative "panel"
require_relative "report"
require_relative "runner"
require_relative "runner/failing"
require_relative "runner/results"
require_relative "runner/settings"
require_relative "script"
require_relative "template"

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
  # What a copy of the project adds to enable a probe and play headless.
  PROBE_SETTINGS = <<~SETTINGS

    [editor_plugins]

    enabled=PackedStringArray("res://addons/probe/plugin.cfg")

    [editor]

    run/main_run_args="--headless"
  SETTINGS
  # The checks, each in the module named after the behaviour it claims.
  CHECKS = [
    Script, Declarations, Report, Runner, Runner::Failing, Runner::Results, Runner::Settings, Loader, Export,
    Panel, Template
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

  # Yields a copy of the project whose editor enables the plugin at `probe`,
  # a directory of the project, and plays its scenes headless, so a probe acts
  # in an editor no person uses and the scenes it plays run where the check
  # does.
  def with_probe(project, probe)
    Dir.mktmpdir do |dir|
      copy = File.join(dir, "project")
      FileUtils.cp_r(project, copy)
      FileUtils.rm_rf(File.join(copy, ".godot", "editor"))
      FileUtils.cp_r(File.join(project, probe), File.join(copy, "addons", "probe"))
      FileUtils.rm(Dir.glob(File.join(copy, "addons", "probe", "*.uid")))
      File.write(File.join(copy, "project.godot"), PROBE_SETTINGS, mode: "a")
      yield copy
    end
  end

  # Opens the project in the editor for this many frames.
  def run_editor_frames(project, frames)
    Open3.capture2e(EXECUTABLE, "--headless", "--editor", "--quit-after", frames, "--path", project)
  end

  # Opens the project in the editor, which scans it, and quits.
  def run_editor(project)
    Open3.capture2e(EXECUTABLE, "--headless", "--editor", "--quit", "--path", project)
  end

  def run_scene(project, scene, *)
    Open3.capture2e(EXECUTABLE, "--headless", "--path", project, scene, *)
  end
end
