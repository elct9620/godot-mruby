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
    OWN_METHOD_LINE = "own method offered: true"
    EXPORT_LINE = "export offered: true"
    ENGINE_METHOD_LINE = "engine method offered: true"
    PROJECT_CONSTANT_LINE = "project constant offered: true"

    module_function

    def verify!(project)
      Godot.copy_probe_project(project, PROBE) do |copy|
        output, = Godot.run_editor_frames(copy, FRAMES)
        verify_engine_class_offered!(output)
        verify_value_type_offered!(output)
        verify_own_method_offered!(output)
        verify_export_offered!(output)
        verify_engine_method_offered!(output)
        verify_project_constant_offered!(output)
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

    # @behavior RQ-010
    def verify_own_method_offered!(output)
      return if output.include?(OWN_METHOD_LINE)

      raise "The script editor offered no method a node script defines after self.:\n#{output}"
    end

    # @behavior RQ-015
    def verify_export_offered!(output)
      return if output.include?(EXPORT_LINE)

      raise "The script editor offered no export a node script declares after self.:\n#{output}"
    end

    # @behavior RQ-011
    def verify_engine_method_offered!(output)
      return if output.include?(ENGINE_METHOD_LINE)

      raise "The script editor offered no method of a node script's engine class after self.:\n#{output}"
    end

    # @behavior RQ-012
    def verify_project_constant_offered!(output)
      return if output.include?(PROJECT_CONSTANT_LINE)

      raise "The script editor offered no constant of the project's top level on a bare name:\n#{output}"
    end
  end
end
