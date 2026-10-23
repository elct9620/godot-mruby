# frozen_string_literal: true

require "fileutils"
require "open3"
require "tmpdir"

require_relative "announcement"
require_relative "completion"
require_relative "conversation"
require_relative "debugger"
require_relative "declarations"
require_relative "documentation"
require_relative "export"
require_relative "highlighting"
require_relative "loader"
require_relative "panel"
require_relative "report"
require_relative "runner"
require_relative "runner/failures"
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
  LOAD_LINE = "Initialize godot-rust"
  ERROR_LINE = /^(ERROR|SCRIPT ERROR):/
  # What a copy of the project adds to enable the probe in a directory and
  # play headless.
  PROBE_SETTINGS = <<~SETTINGS

    [editor_plugins]

    enabled=PackedStringArray("res://%s/plugin.cfg")

    [editor]

    run/main_run_args="--headless"
  SETTINGS
  # Every run of Godot a check makes, with what it printed, so a failed check
  # leaves the whole output however little its message quotes; CI uploads
  # TEST_REPORT_DIR when a job fails.
  RUN_LOG = File.join(ENV.fetch("TEST_REPORT_DIR", Dir.tmpdir), "godot-runs.log")
  # The checks, each in the module named after the behaviour it claims.
  CHECKS = [
    Script, Declarations, Report, Runner, Runner::Failures, Runner::Results, Runner::Settings, Loader, Export,
    Panel, Template, Announcement, Highlighting, Completion, Documentation, Debugger
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

  # Starts from no editor layout: one a person or an earlier run left names
  # the scenes and scripts to open, and restoring them runs their tool
  # scripts, such as the probes' fixtures that do not parse, before any check.
  # Nor does it keep an earlier verify's run of the Ruby tests.
  def verify!(project = PROJECT)
    FileUtils.rm_rf(File.join(project, ".godot", "editor"))
    Runner.forget_runs
    log_runs do
      verify_loaded!(project)
      CHECKS.each do |check|
        report_start(check)
        check.verify!(project)
      end
    end
  end

  # Names `check` as it starts, so a run that stops shows where.
  def report_start(check)
    puts "Checking #{check.name.delete_prefix("Godot::")}"
    $stdout.flush
  end

  # Writes the runs the checks in the block make to RUN_LOG, started afresh,
  # and names it in the error of the check that fails.
  def log_runs
    FileUtils.rm_f(RUN_LOG)
    @logged = true
    yield
  rescue StandardError => e
    raise e.exception("#{e.message}\nEvery run's output is in #{RUN_LOG}")
  ensure
    @logged = false
  end

  # Runs `command` as Open3.capture2e does, adding the run to RUN_LOG inside
  # log_runs; outside it, as for the bench, a run is timed and kept nowhere.
  # `respond`, when given, keeps the run's input open and is handed all it
  # has printed each time more arrives, answering what to type next or nil.
  def capture(*command, stdin_data: "", respond: nil, **)
    output, status = Conversation.hold(command, stdin_data, respond, **)
    log_run(command, output, status || "killed as hung")
    raise "#{command.join(" ")} was killed as hung, ending:\n#{output.lines.last(20).join}" unless status

    [output, status]
  end

  def log_run(command, output, status)
    return unless @logged

    FileUtils.mkdir_p(File.dirname(RUN_LOG))
    File.write(RUN_LOG, "$ #{command.join(" ")}\n#{output}#{status}\n\n", mode: "a")
  end

  # The project keeps its extension list, so the editor loads the addon at
  # startup instead of discovering it: an editor that discovers an extension
  # with classes and quits at once crashes (godotengine/godot#111048). Godot
  # reports a library it could not open on stdout and may still exit 0, so the
  # pass is judged by what it printed.
  # @behavior RA-001
  def verify_loaded!(project = PROJECT)
    output, status = run_editor(project)
    errors = output.lines.grep(ERROR_LINE)
    return if status.success? && errors.empty? && output.include?(LOAD_LINE)

    raise "The extension did not load:\n#{errors.empty? ? output : errors.join}"
  end

  # Yields a copy of the project whose editor enables the plugin in `probe`,
  # a directory of the project, where it stands, so a Ruby plugin keeps the
  # name its path spells; the copy plays its scenes headless, so a probe acts
  # in an editor no person uses and the scenes it plays run where the check
  # does.
  def copy_probe_project(project, probe)
    Dir.mktmpdir do |dir|
      copy = File.join(dir, "project")
      FileUtils.cp_r(project, copy)
      FileUtils.rm_rf(File.join(copy, ".godot", "editor"))
      File.write(File.join(copy, "project.godot"), format(PROBE_SETTINGS, probe), mode: "a")
      yield copy
    end
  end

  # Opens the project in the editor for this many frames.
  def run_editor_frames(project, frames)
    capture(EXECUTABLE, "--headless", "--editor", "--quit-after", frames, "--path", project)
  end

  # Opens the project in the editor, which scans it, and quits.
  def run_editor(project)
    capture(EXECUTABLE, "--headless", "--editor", "--quit", "--path", project)
  end

  def run_scene(project, scene, *, **)
    capture(EXECUTABLE, "--headless", "--path", project, scene, *, **)
  end
end
