# Opens a Ruby script in the script editor, as a person would, and prints the
# highlighter the editor chose for it and whether its `def` takes the theme's
# keyword colour, then again once that line is replaced by an assignment.
module Integration
  module Highlighting
    module Probe
      class HighlightingProbe < Godot::EditorPlugin
        tool

        PATH = "res://integration/highlighting/probe/speed.rb".freeze
        OPEN_AT = 30
        LOOK_AT = 40

        def _ready
          @frames = 0
        end

        def _process(_delta)
          @frames += 1
          if @frames == OPEN_AT
            Godot::EditorInterface.edit_script(Godot::ResourceLoader.load(PATH))
          elsif @frames == LOOK_AT
            look
          end
        end

        def look
          code = Godot::EditorInterface.get_script_editor.get_current_editor.get_base_editor
          highlighter = code.syntax_highlighter
          puts "highlighter chosen: #{highlighter.get_class}"
          keyword = Godot::EditorInterface.get_editor_settings
                                          .get_setting("text_editor/theme/highlighting/keyword_color")
          puts "keyword coloured: #{read_first_color(highlighter) == keyword}"
          code.set_line(1, "speed = 1")
          puts "edited keyword coloured: #{read_first_color(highlighter) == keyword}"
        end

        def read_first_color(highlighter)
          first = highlighter.get_line_syntax_highlighting(1)[0]
          first && first["color"]
        end
      end
    end
  end
end
