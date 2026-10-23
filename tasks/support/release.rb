# frozen_string_literal: true

require "fileutils"
require "tmpdir"

require_relative "debugger"
require_relative "declarations"
require_relative "engine_classes"
require_relative "export"
require_relative "values"

module Godot
  # Plays the project's release checks on export templates: on the release
  # build, which leaves out the engine's debug checks, so what the extension
  # refuses in their place shows, and on the debug build, whose library alone
  # of the two stops at breakpoints. GODOT_RELEASE_TEMPLATE and
  # GODOT_DEBUG_TEMPLATE name the binaries from Godot's export templates
  # archive for this platform, such as `linux_release.x86_64` and
  # `linux_debug.x86_64`. A template takes neither a pack nor a scene on its
  # command line: it runs `game.pck` beside it, and the scene an
  # `override.cfg` there names.
  module Release
    TEMPLATE = "GODOT_RELEASE_TEMPLATE"
    DEBUG_TEMPLATE = "GODOT_DEBUG_TEMPLATE"
    GAME = "game"
    # The scenes quit as they finish; this only stops one that never does.
    FRAMES = "60"
    CHECKS = [Declarations, EngineClasses, Export, Values, Debugger].freeze
    DEBUG_CHECKS = [Debugger].freeze

    module_function

    def verify!(project = PROJECT)
      Godot.log_runs do
        install_game(project, TEMPLATE) { |dir| play_checks(CHECKS, :verify_release!, dir) }
        install_game(project, DEBUG_TEMPLATE) { |dir| play_checks(DEBUG_CHECKS, :verify_debug!, dir) }
      end
    end

    def play_checks(checks, verify, dir)
      checks.each do |check|
        Godot.report_start(check)
        check.public_send(verify, dir)
      end
    end

    # Runs the block with a directory holding the project exported for the
    # template ENV[variable] names.
    def install_game(project, variable)
      template = ENV.fetch(variable) { raise "#{variable} names no export template's binary" }
      Dir.mktmpdir do |dir|
        FileUtils.cp(template, File.join(dir, GAME))
        Export.export_pack(project, dir)
        Export.install_library(project, dir)
        yield dir
      end
    end

    # Plays `scene` of the game in `dir`, with the arguments and what `capture`
    # takes besides, and answers what it printed and how it ended.
    def play(dir, scene, *, **)
      File.write(File.join(dir, "override.cfg"), "[application]\n\nrun/main_scene=#{scene.dump}\n")
      Godot.capture(File.join(dir, GAME), "--headless", "--quit-after", FRAMES, *, chdir: dir, **)
    end

    # The messages of the refusals `scene` printed, or the error naming what
    # it printed when it did not end cleanly, as a game the engine stopped
    # does not.
    def collect_refusals(dir, scene)
      output, status = play(dir, scene)
      raise "#{scene} did not end cleanly on the release build:\n#{output}" unless status.success?

      output.lines.filter_map { |line| line.chomp.delete_prefix("refused: ") if line.start_with?("refused: ") }
    end
  end
end
