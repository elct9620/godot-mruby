# frozen_string_literal: true

module Godot
  # Opens a Ruby script in the script editor of a copy of the
  # integration-test project and reads how the editor coloured it. Part of
  # Godot.verify!.
  module Highlighting
    PROBE = File.join("integration", "highlighting", "probe")
    FRAMES = "60"
    CHOSEN = "highlighter chosen: RubySyntaxHighlighter"
    COLOURED = "keyword coloured: true"
    RECOLOURED = "edited keyword coloured: false"

    module_function

    def verify!(project)
      Godot.with_probe(project, PROBE) do |copy|
        output, = Godot.run_editor_frames(copy, FRAMES)
        verify_coloured!(output)
        verify_recoloured!(output)
      end
    end

    # @behavior RU-013
    def verify_coloured!(output)
      return if output.include?(CHOSEN) && output.include?(COLOURED)

      raise "The script editor did not colour a Ruby script's keyword with the Ruby highlighter:\n#{output}"
    end

    # @behavior RU-014
    def verify_recoloured!(output)
      return if output.include?(RECOLOURED)

      raise "The script editor kept a keyword's colour on a line edited into an assignment:\n#{output}"
    end
  end
end
