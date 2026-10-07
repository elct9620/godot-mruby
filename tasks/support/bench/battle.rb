# frozen_string_literal: true

require "fileutils"
require "tmpdir"

require_relative "../godot"
require_relative "project"

module Bench
  # Plays the battle of the game in godot/src against its GDScript twin, in
  # each style the call-cost analysis compares, and answers what a round
  # costs each language and how many times GDScript's Ruby takes. The
  # call-cost gate marks either style's ratio grown past main's; this only
  # reports. A pair whose two battles do not end alike fails, since a twin
  # that drifted from the game times something else.
  module Battle
    TEMPLATE = File.expand_path("../../bench/battle", __dir__)
    # The game's battle as written, and as the analysis recommends writing it.
    STYLES = %w[original recommended].freeze
    SCENES = { ruby: "res://src/battle.tscn", gdscript: "res://gd/battle.tscn" }.freeze
    WAVE = 100
    ROUNDS = 10
    # Ruby and GDScript play in turns this many times, so both meet the same
    # load on a shared machine.
    PAIRS = 5
    # Far more physics frames than ROUNDS rounds take, so a battle that never
    # ends stops rather than holding the run.
    FRAME_LIMIT = "20000"
    # What the measure scene prints once the rounds are over.
    OUTCOME = /^battle frames=(\d+) usec=(\d+) breached=(\w+)$/

    module_function

    # Each style's round in milliseconds for each language, and the ratio,
    # each as the median of the pairs and their range.
    def measure(source = Godot::PROJECT)
      STYLES.to_h do |style|
        Dir.mktmpdir do |project|
          generate(project, style, source)
          Godot.run_editor(project)
          [style, summarize_pairs(Array.new(PAIRS) { play_pair(project) })]
        end
      end
    end

    # Writes the battle project of `style`: the shared template, the game of
    # `source`, and the files the style writes over them.
    def generate(project, style, source)
      Project.install(project, source, File.join(TEMPLATE, "base"))
      FileUtils.cp_r(File.join(source, "src"), project)
      FileUtils.cp_r(File.join(TEMPLATE, style, "."), project)
    end

    # One round's milliseconds in each language, Ruby first, after checking
    # the two battles ended alike.
    def play_pair(project)
      ruby, gdscript = SCENES.values.map { |scene| play(project, scene) }
      unless ruby.except(:usec) == gdscript.except(:usec)
        raise "The battles ended apart: Ruby #{ruby}, GDScript #{gdscript}"
      end

      [ruby, gdscript].map { |outcome| outcome[:usec] / 1000.0 / ROUNDS }
    end

    # The frames, microseconds and outcome of ROUNDS rounds of `scene`.
    def play(project, scene)
      output, = Godot.run_scene(project, "res://measure.tscn", "--fixed-fps", "60", "--quit-after", FRAME_LIMIT,
                                "--", scene, WAVE.to_s, ROUNDS.to_s)
      outcome = output.match(OUTCOME)
      raise "The battle #{scene} did not finish:\n#{output}" unless outcome

      frames, usec, breached = outcome.captures
      { frames: Integer(frames), usec: Integer(usec), breached: breached }
    end

    def summarize_pairs(pairs)
      {
        "ruby" => summarize_values(pairs.map(&:first)),
        "gdscript" => summarize_values(pairs.map(&:last)),
        "ratio" => summarize_values(pairs.map { |ruby, gdscript| ruby / gdscript })
      }
    end

    # The median of `values` and their range.
    def summarize_values(values)
      sorted = values.sort
      { "median" => sorted[sorted.size / 2], "min" => sorted.first, "max" => sorted.last }
    end
  end
end
