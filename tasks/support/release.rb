# frozen_string_literal: true

require "fileutils"
require "open3"
require "tmpdir"

require_relative "engine_classes"
require_relative "export"
require_relative "values"

module Godot
  # Plays the project's release checks on an export template's release
  # build, which leaves out the engine's debug checks, so what the extension
  # refuses in their place shows. GODOT_RELEASE_TEMPLATE names the release
  # binary from Godot's export templates archive for this platform, such as
  # `linux_release.x86_64`. A template takes neither a pack nor a scene on its
  # command line: it runs `game.pck` beside it, and the scene an
  # `override.cfg` there names.
  module Release
    TEMPLATE = "GODOT_RELEASE_TEMPLATE"
    GAME = "game"
    # The scenes quit as they finish; this only stops one that never does.
    FRAMES = "60"
    CHECKS = [EngineClasses, Values].freeze

    module_function

    def verify!(project = PROJECT)
      template = ENV.fetch(TEMPLATE) { raise "#{TEMPLATE} names no export template's release binary" }
      Dir.mktmpdir do |dir|
        FileUtils.cp(template, File.join(dir, GAME))
        Export.export_pack(project, dir)
        Export.install_library(project, dir)
        CHECKS.each { |check| check.verify_release!(dir) }
      end
    end

    # Plays `scene` of the game in `dir` and answers what it printed and how
    # it ended.
    def play(dir, scene)
      File.write(File.join(dir, "override.cfg"), "[application]\n\nrun/main_scene=#{scene.dump}\n")
      Open3.capture2e(File.join(dir, GAME), "--headless", "--quit-after", FRAMES, chdir: dir)
    end

    # The messages of the refusals `scene` printed, or the error naming what
    # it printed when it did not end cleanly, as a game the engine stopped
    # does not.
    def refusals(dir, scene)
      output, status = play(dir, scene)
      raise "#{scene} did not end cleanly on the release build:\n#{output}" unless status.success?

      output.lines.filter_map { |line| line.chomp.delete_prefix("refused: ") if line.start_with?("refused: ") }
    end
  end
end
