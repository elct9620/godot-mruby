# frozen_string_literal: true

require "fileutils"
require "open3"
require "tmpdir"

module Godot
  # Exports the project as a pack and runs it as the exported game, reading
  # what it printed. No export template is needed: the editor's own build runs
  # the pack, and the project's export preset declares `template`, the feature
  # an export template's build carries, so the game answers as a shipped one.
  # Part of Godot.verify!.
  module ExportedGame
    PRESET = "Verify"
    LEFTOVER_SCENE = "res://verify/export/leftover.tscn"
    # What the leftover scene loads under a test directory, as the constant
    # its path names.
    LEFTOVER = "module Test\n  class Leftover\n  end\nend\n"
    LEFT_OUT = "left out: uninitialized constant Test"
    # The run ends within its first frames; this only stops one that never
    # does from hanging the check.
    FRAMES = "60"

    module_function

    # @behavior RX-001
    def verify!(project)
      Dir.mktmpdir do |dir|
        pack = export_pack(project, dir)
        source = File.join(dir, "leftover.rb")
        File.write(source, LEFTOVER)
        output = run(project, dir, pack, LEFTOVER_SCENE, source, File.join(dir, "leftover.pck"))
        next if output.lines.map(&:chomp).include?(LEFT_OUT)

        raise "The exported game's Ruby reached a file under a test directory:\n#{output}"
      end
    end

    # The export prints the editor's complaints about having no window, so it
    # is judged by the pack it leaves.
    def export_pack(project, dir)
      pack = File.join(dir, "game.pck")
      output, = Open3.capture2e(EXECUTABLE, "--headless", "--path", project, "--export-pack", PRESET, pack)
      raise "The project did not export as a pack:\n#{output}" unless File.exist?(pack)

      pack
    end

    # A pack carries no library, so the game runs from a directory that holds
    # the addon's at the path the .gdextension names.
    def run(project, dir, pack, scene, *)
      bin = File.join("addons", "godot_mruby", "bin")
      FileUtils.mkdir_p(File.join(dir, File.dirname(bin)))
      FileUtils.cp_r(File.join(project, bin), File.join(dir, bin))
      output, status = Open3.capture2e(EXECUTABLE, "--headless", "--main-pack", pack, scene,
                                       "--quit-after", FRAMES, "--", *, chdir: dir)
      raise "The exported game did not run:\n#{output}" unless status.success?

      output
    end
  end
end
