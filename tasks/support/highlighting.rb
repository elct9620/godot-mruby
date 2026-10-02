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

    module_function

    # @behavior RU-013
    def verify!(project)
      Godot.with_probe(project, PROBE) do |copy|
        output, = Godot.run_editor_frames(copy, FRAMES)
        next if output.include?(CHOSEN) && output.include?(COLOURED)

        raise "The script editor did not colour a Ruby script's keyword with the Ruby highlighter:\n#{output}"
      end
    end
  end
end
