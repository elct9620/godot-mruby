# frozen_string_literal: true

module Godot
  # Opens a Ruby script in the script editor of a copy of the
  # integration-test project and reads how the editor coloured it. Part of
  # Godot.verify!.
  module Highlighting
    PROBE = File.join("integration", "highlighting", "probe")
    FRAMES = "60"
    CHOICE_LINE = "highlighter chosen: RubySyntaxHighlighter"
    COLOUR_LINE = "keyword coloured: true"
    RECOLOUR_LINE = "edited keyword coloured: false"

    module_function

    def verify!(project)
      Godot.copy_probe_project(project, PROBE) do |copy|
        output, = Godot.run_editor_frames(copy, FRAMES)
        verify_coloured!(output)
        verify_recoloured!(output)
      end
    end

    # @behavior RU-013
    def verify_coloured!(output)
      return if output.include?(CHOICE_LINE) && output.include?(COLOUR_LINE)

      raise "The script editor did not colour a Ruby script's keyword with the Ruby highlighter:\n#{output}"
    end

    # @behavior RU-014
    def verify_recoloured!(output)
      return if output.include?(RECOLOUR_LINE)

      raise "The script editor kept a keyword's colour on a line edited into an assignment:\n#{output}"
    end
  end
end
