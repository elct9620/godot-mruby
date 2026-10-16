# frozen_string_literal: true

module Godot
  # Opens a Ruby node script's page in the editor's help of a copy of the
  # integration-test project and reads whether its comments were shown. Part
  # of Godot.verify!.
  module Documentation
    PROBE = File.join("integration", "documentation", "probe")
    FRAMES = "70"
    CLASS_COMMENT_LINE = "class comment shown: true"
    METHOD_COMMENT_LINE = "method comment shown: true"

    module_function

    def verify!(project)
      Godot.copy_probe_project(project, PROBE) do |copy|
        output, = Godot.run_editor_frames(copy, FRAMES)
        verify_class_comment_shown!(output)
        verify_method_comment_shown!(output)
      end
    end

    # @behavior RZ-005
    def verify_class_comment_shown!(output)
      return if output.include?(CLASS_COMMENT_LINE)

      raise "The editor's help did not show a Ruby class's comment on its page:\n#{output}"
    end

    # @behavior RZ-006
    def verify_method_comment_shown!(output)
      return if output.include?(METHOD_COMMENT_LINE)

      raise "The editor's help did not show a Ruby method's comment on its class's page:\n#{output}"
    end
  end
end
