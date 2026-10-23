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
    SCENES = {
      ruby: "res://operations/operations_ruby.tscn",
      gdscript: "res://operations/operations_gdscript.tscn"
    }.freeze
    TICKERS = { ruby: "res://src/ticker.rb", gdscript: "res://operations/ticker.gd" }.freeze
    CALLBACK = "res://operations/callback.tscn"
    # The calls the scenes time, as they print them.
    CALLS = %w[loop ruby_call engine_call position_get position_set member_set singleton_call utility_call
               vector_new color_new vector_add vector_x callback].freeze
    TIMING = /^op (\w+) ([\d.]+)$/
    # Frames past which a scene that never quits is stopped: the op scenes
    # time everything as they become ready, the callback scene in 100 frames.
    FRAME_LIMITS = { operations: "10", callback: "1000" }.freeze
    # Godot's command-line debugger, which every run from the editor has in
    # its remote form: it holds no breakpoint, and a break nothing expected
    # quits rather than waiting for input.
    UNDER_DEBUGGER = { arguments: ["-d"], options: { stdin_data: "q\n" } }.freeze
    PLAIN = { arguments: [], options: {} }.freeze

    module_function

    # Each call's nanoseconds in each language and the ratio, each as the
    # median of the pairs and their range, played as `run` says.
    def measure(source = Godot::PROJECT, run: PLAIN)
      Dir.mktmpdir do |project|
        Battle.generate(project, "original", source)
        Godot.run_editor(project)
        pairs = Array.new(Battle::PAIRS) { LANGUAGES.map { |language| time_calls(project, language, run) } }
        CALLS.to_h { |call| [call, summarize_call(pairs, call)] }
      end
    end

    def summarize_call(pairs, call)
      Battle.summarize_pairs(pairs.map { |ruby, gdscript| [ruby[call], gdscript[call]] })
    end

    # The nanoseconds of one call of each kind in `language`.
    def time_calls(project, language, run = PLAIN)
      timed = play(project, SCENES[language], run, "--quit-after", FRAME_LIMITS[:operations])
      callback = play(project, CALLBACK, run, "--fixed-fps", "60", "--quit-after", FRAME_LIMITS[:callback],
                      "--", TICKERS[language])
      output = timed + callback
      timings = output.scan(TIMING).to_h { |call, nanoseconds| [call, Float(nanoseconds)] }
      raise "The #{language} calls were not all timed:\n#{output}" unless timings.keys.sort == CALLS.sort

      timings
    end

    # What `scene` printed, played with the arguments as `run` says.
    def play(project, scene, run, *)
      output, = Godot.run_scene(project, scene, *run[:arguments], *, **run[:options])
      output
    end
  end
end
