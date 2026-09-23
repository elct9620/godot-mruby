# frozen_string_literal: true

require "fileutils"
require "open3"
require "tmpdir"

module Godot
  # Exports the project as a pack and runs it as the exported game, reading
  # what it printed. No export template is needed: the editor's own build runs
  # the pack, and the project's export preset declares `template`, the feature
  # an export template's build carries, so the game answers as a shipped one.
  # That build also carries `editor`, which an export template's does not, so
  # a game property still comes with its category here; no check reads one.
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
    SHIPPED_SCENE = "res://verify/export/shipped.tscn"
    # Paths an export has to leave out, and a game file it has to ship, which
    # tells a game missing everything from one missing only its tests.
    LEFT_OUT_PATHS = ["res://test/engine_classes/engine_classes_test.rb",
                      "res://addons/godot_mruby/runner.tscn"].freeze
    GAME_FILE = "res://verify/export/seeker.rb"
    REFUSED_SCENE = "res://verify/export/refused.tscn"
    REFUSED = "ERROR: The test runner does not run in an exported game"

    module_function

    def verify!(project)
      Dir.mktmpdir do |dir|
        pack = export_pack(project, dir)
        install_library(project, dir)
        verify_index_left_out!(dir, pack)
        verify_tests_left_out!(dir, pack)
        verify_runner_refused!(dir, pack)
      end
    end

    # @behavior RX-001
    def verify_index_left_out!(dir, pack)
      source = File.join(dir, "leftover.rb")
      File.write(source, LEFTOVER)
      output, status = run(dir, pack, LEFTOVER_SCENE, source, File.join(dir, "leftover.pck"))
      return if status.success? && output.lines.map(&:chomp).include?(LEFT_OUT)

      raise "The exported game's Ruby reached a file under a test directory:\n#{output}"
    end

    # @behavior RX-002 RX-003
    def verify_tests_left_out!(dir, pack)
      output, status = run(dir, pack, SHIPPED_SCENE, *LEFT_OUT_PATHS, GAME_FILE)
      shipped = output.lines.filter_map do |line|
        line.chomp.delete_prefix("shipped: ") if line.start_with?("shipped: ")
      end
      return if status.success? && shipped == [GAME_FILE]

      raise "The exported game shipped #{shipped} where only #{GAME_FILE} belongs:\n#{output}"
    end

    # The runner quits the game with 1 as it refuses.
    # @behavior RX-006
    def verify_runner_refused!(dir, pack)
      output, status = run(dir, pack, REFUSED_SCENE)
      return if status.exitstatus == 1 && output.lines.map(&:chomp).include?(REFUSED)

      raise "The exported game ran a test runner:\n#{output}"
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
    def install_library(project, dir)
      bin = File.join("addons", "godot_mruby", "bin")
      FileUtils.mkdir_p(File.join(dir, File.dirname(bin)))
      FileUtils.cp_r(File.join(project, bin), File.join(dir, bin))
    end

    def run(dir, pack, scene, *)
      Open3.capture2e(EXECUTABLE, "--headless", "--main-pack", pack, scene,
                      "--quit-after", FRAMES, "--", *, chdir: dir)
    end
  end
end
