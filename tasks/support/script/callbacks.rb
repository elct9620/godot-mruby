# frozen_string_literal: true

module Godot
  module Script
    # Runs the integration-test project's scene of node scripts whose classes
    # define callbacks and methods Godot calls, and reads what they print: which
    # callbacks a node's Ruby object is given, how often each object is built,
    # and what its methods answer Godot. Part of Script.verify!.
    module Callbacks
      # The scene, what its scripts print, what they must never print, and the
      # answers Godot gets.
      SCENE = "res://integration/script/callbacks/callbacks.tscn"
      READY_LINE = "ready.rb is ready"
      PROCESS_LINE = "process.rb was given a Float"
      BUILT_LINE = "built.rb built an object"
      FAILED_LINE = "SCRIPT ERROR: failing.rb cannot be built"
      FAILED_PROCESS_LINE = "failing.rb processed"
      IDLE_LINE = "idle.rb ran"
      NOTIFIED_LINE = "notified.rb was notified it is ready"
      ANSWER_LINES = ["ratio answered float 1.5", "count answered int 3"].freeze
      # What Godot gets for an answer that cannot cross, and the log's reason.
      REFUSED_ANSWER_LINES = ["looped answered Nil <null>",
                              "ERROR: #looped answered [[...]]: an Array nested more than 100 deep cannot " \
                              "reach the engine"].freeze
      # What the log says of an argument that cannot cross, and the line the
      # method would print were it called.
      REFUSED_ARGUMENT_LINE = "ERROR: #take was not called: an Array nested more than 100 deep cannot reach Ruby"
      TAKEN_LINE = "take was called"
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
        verify_refused_answer!(lines, output)
        verify_refused_argument!(lines, output)
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

      # @behavior RV-008
      def verify_refused_answer!(lines, output)
        missing = REFUSED_ANSWER_LINES.reject { |line| lines.include?(line) }
        return if missing.empty?

        raise "The callbacks scene's answer that cannot cross did not reach Godot as null #{missing}:\n#{output}"
      end

      # @behavior RV-009
      def verify_refused_argument!(lines, output)
        return if lines.include?(REFUSED_ARGUMENT_LINE) && !lines.include?(TAKEN_LINE)

        raise "The callbacks scene's argument that cannot cross reached Ruby:\n#{output}"
      end
    end
  end
end
