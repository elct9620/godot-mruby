# frozen_string_literal: true

module Godot
  # Types into a Ruby script in the script editor of a copy of the
  # integration-test project and reads the names the editor offered, and
  # where looking names up took it. Part of Godot.verify!.
  module Completion
    PROBE = File.join("integration", "completion", "probe")
    FRAMES = "80"
    ENGINE_CLASS_LINE = "engine class offered: true"
    VALUE_TYPE_LINE = "value type offered: true"
    OWN_METHOD_LINE = "own method offered: true"
    EXPORT_LINE = "export offered: true"
    ENGINE_METHOD_LINE = "engine method offered: true"
    PROJECT_CONSTANT_LINE = "project constant offered: true"
    METHOD_LOOKUP_LINE = "method looked up: true"
    ENGINE_LOOKUP_LINE = "engine member looked up: true"
    CONSTANT_LOOKUP_LINE = "constant looked up: true"

    module_function

    def verify!(project)
      Godot.copy_probe_project(project, PROBE) do |copy|
        output, = Godot.run_editor_frames(copy, FRAMES)
        verify_offers!(output)
        verify_lookups!(output)
      end
    end

    def verify_offers!(output)
      verify_engine_class_offered!(output)
      verify_value_type_offered!(output)
      verify_own_method_offered!(output)
      verify_export_offered!(output)
      verify_engine_method_offered!(output)
      verify_project_constant_offered!(output)
    end

    def verify_lookups!(output)
      verify_method_looked_up!(output)
      verify_engine_member_looked_up!(output)
      verify_constant_looked_up!(output)
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

    # @behavior RQ-020
    def verify_method_looked_up!(output)
      return if output.include?(METHOD_LOOKUP_LINE)

      raise "Looking up a node script's method did not reach its def line:\n#{output}"
    end

    # @behavior RQ-021
    def verify_engine_member_looked_up!(output)
      return if output.include?(ENGINE_LOOKUP_LINE)

      raise "Looking up an engine method did not open its class's help page:\n#{output}"
    end

    # @behavior RQ-022
    def verify_constant_looked_up!(output)
      return if output.include?(CONSTANT_LOOKUP_LINE)

      raise "Looking up a library constant did not open its file at the statement naming it:\n#{output}"
    end
  end
end
