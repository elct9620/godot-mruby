# frozen_string_literal: true

require "tmpdir"

require_relative "../godot"
require_relative "battle"

module Bench
  # Times each kind of call a game makes, in Ruby and in its GDScript twin,
  # in a battle project, and answers the nanoseconds of one call and the
  # ratio, so a change to one kind shows in its own row. Each time includes
  # its loop, whose own time is a row too.
  module Operations
    LANGUAGES = %i[ruby gdscript].freeze
    SCENES = { ruby: "res://ops/ops_ruby.tscn", gdscript: "res://ops/ops_gdscript.tscn" }.freeze
    TICKERS = { ruby: "res://src/ticker.rb", gdscript: "res://ops/ticker.gd" }.freeze
    CALLBACK = "res://ops/callback.tscn"
    # The calls the scenes time, as they print them.
    CALLS = %w[loop ruby_call engine_call position_get position_set vector_new color_new vector_add vector_x
               callback].freeze
    TIMING = /^op (\w+) ([\d.]+)$/
    # Frames past which a scene that never quits is stopped: the op scenes
    # time everything as they become ready, the callback scene in 100 frames.
    FRAME_LIMITS = { ops: "10", callback: "1000" }.freeze

    module_function

    # Each call's nanoseconds in each language and the ratio, each as the
    # median of the pairs and their range.
    def measure(source = Godot::PROJECT)
      Dir.mktmpdir do |project|
        Battle.generate(project, "original", source)
        Godot.run_editor(project)
        pairs = Array.new(Battle::PAIRS) { LANGUAGES.map { |language| time_calls(project, language) } }
        CALLS.to_h { |call| [call, summarize_call(pairs, call)] }
      end
    end

    def summarize_call(pairs, call)
      Battle.summarize_pairs(pairs.map { |ruby, gdscript| [ruby[call], gdscript[call]] })
    end

    # The nanoseconds of one call of each kind in `language`.
    def time_calls(project, language)
      ops, = Godot.run_scene(project, SCENES[language], "--quit-after", FRAME_LIMITS[:ops])
      callback, = Godot.run_scene(project, CALLBACK, "--fixed-fps", "60", "--quit-after", FRAME_LIMITS[:callback],
                                  "--", TICKERS[language])
      output = ops + callback
      timings = output.scan(TIMING).to_h { |call, nanoseconds| [call, Float(nanoseconds)] }
      raise "The #{language} calls were not all timed:\n#{output}" unless timings.keys.sort == CALLS.sort

      timings
    end
  end
end
