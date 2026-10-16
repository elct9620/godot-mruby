# frozen_string_literal: true

module Godot
  # Types into a Ruby script in the script editor of a copy of the
  # integration-test project and reads the names the editor offered. Part of
  # Godot.verify!.
  module Completion
    PROBE = File.join("integration", "completion", "probe")
    FRAMES = "60"
    ENGINE_CLASS_LINE = "engine class offered: true"
    VALUE_TYPE_LINE = "value type offered: true"

    module_function

    def verify!(project)
      Godot.copy_probe_project(project, PROBE) do |copy|
        output, = Godot.run_editor_frames(copy, FRAMES)
        verify_engine_class_offered!(output)
        verify_value_type_offered!(output)
      end
    end

    # @behavior RQ-003
    def verify_engine_class_offered!(output)
      return if output.include?(ENGINE_CLASS_LINE)

      raise "The script editor offered no engine class after Godot:: in a Ruby script:\n#{output}"
    end

    # @behavior RQ-004
    def verify_value_type_offered!(output)
      return if output.include?(VALUE_TYPE_LINE)

      raise "The script editor offered no value type after Godot:: in a Ruby script:\n#{output}"
    end
  end
end
