# frozen_string_literal: true

module Godot
  module Script
    # Runs the integration-test project's scene of node scripts whose classes
    # define callbacks and methods Godot calls, and reads what they print: how
    # often each object is built, and what the log says of an answer that
    # cannot cross from a method. Part of Script.verify!.
    module Callbacks
      # The scene, what its scripts print, and what they must never print.
      SCENE = "res://integration/script/callbacks/callbacks.tscn"
      BUILT_LINE = "brick.rb built an object"
      FAILED_LINE = "SCRIPT ERROR: fault.rb cannot be built"
      FAILED_PROCESS_LINE = "fault.rb processed"
      IDLE_LINE = "statue.rb ran"
      # What Godot gets for an answer that cannot cross, and the log's reason.
      REFUSED_ANSWER_LINES = ["make_self_loop answered Nil <null>",
                              "ERROR: #make_self_loop answered [[...]]: an Array nested more than 100 deep cannot " \
                              "reach the engine"].freeze

      module_function

      # Runs the callbacks scene for a few frames and counts what its Ruby printed.
      def verify!(project)
        output, status = Godot.run_scene(project, SCENE, "--quit-after", "5")
        raise "The callbacks scene did not run:\n#{output}" unless status.success?

        lines = output.lines.map(&:chomp)
        verify_objects_built!(lines, output)
        verify_refused_answer!(lines, output)
      end

      # @behavior RS-013 RS-014 RS-015 RS-016
      def verify_objects_built!(lines, output)
        counts = { BUILT_LINE => 2, FAILED_LINE => 1, FAILED_PROCESS_LINE => 0, IDLE_LINE => 0 }
        wrong = counts.reject { |line, count| lines.count(line) == count }
        return if wrong.empty?

        raise "The callbacks scene's objects were not built as they should be #{wrong.keys}:\n#{output}"
      end

      # @behavior RV-008
      def verify_refused_answer!(lines, output)
        missing = REFUSED_ANSWER_LINES.reject { |line| lines.include?(line) }
        return if missing.empty?

        raise "The callbacks scene's answer that cannot cross did not reach Godot as null #{missing}:\n#{output}"
      end
    end
  end
end
