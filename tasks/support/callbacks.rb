# frozen_string_literal: true

module Godot
  # Runs the integration-test project's scene of node scripts whose classes
  # define callbacks and methods Godot calls, and reads what they print: which
  # callbacks a node's Ruby object is given, how often each object is built,
  # and what its methods answer Godot. Part of NodeScripts.verify!.
  module Callbacks
    # The scene, what its scripts print, what they must never print, and the
    # answers Godot gets.
    SCENE = "res://verify/script/callbacks/callbacks.tscn"
    READY_LINE = "ready.rb is ready"
    PROCESS_LINE = "process.rb was given a Float"
    BUILT_LINE = "built.rb built an object"
    FAILED_LINE = "SCRIPT ERROR: failing.rb cannot be built"
    FAILED_PROCESS_LINE = "failing.rb processed"
    IDLE_LINE = "idle.rb ran"
    NOTIFIED_LINE = "notified.rb was notified it is ready"
    ANSWER_LINES = ["ratio answered float 1.5", "count answered int 3"].freeze
    ITSELF_LINE = "itself.rb is inside the tree: true"

    module_function

    # Runs the callbacks scene for a few frames and counts what its Ruby printed.
    def verify!(project)
      output, status = Godot.run_scene(project, SCENE, "--quit-after", "5")
      raise "The callbacks scene did not run:\n#{output}" unless status.success?

      lines = output.lines.map(&:chomp)
      verify_callbacks_called!(lines, output)
      verify_objects_built!(lines, output)
      verify_answers_reached!(lines, output)
    end

    # @behavior RS-011 RS-012 RS-017 RS-030
    def verify_callbacks_called!(lines, output)
      missing = [READY_LINE, PROCESS_LINE, NOTIFIED_LINE, ITSELF_LINE].reject { |line| lines.include?(line) }
      return if missing.empty?

      raise "The callbacks scene's callbacks were not called #{missing}:\n#{output}"
    end

    # @behavior RS-013 RS-014 RS-015 RS-016
    def verify_objects_built!(lines, output)
      counts = { BUILT_LINE => 2, FAILED_LINE => 1, FAILED_PROCESS_LINE => 0, IDLE_LINE => 0 }
      wrong = counts.reject { |line, count| lines.count(line) == count }
      return if wrong.empty?

      raise "The callbacks scene's objects were not built as they should be #{wrong.keys}:\n#{output}"
    end

    # @behavior RS-028
    def verify_answers_reached!(lines, output)
      missing = ANSWER_LINES.reject { |line| lines.include?(line) }
      return if missing.empty?

      raise "The callbacks scene's answers did not reach Godot as Ruby gave them #{missing}:\n#{output}"
    end
  end
end
