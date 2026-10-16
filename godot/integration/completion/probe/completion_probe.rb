# Opens a node script in the script editor, as a person would, types a name
# on its blank line, asks for completion and prints whether the editor
# offers what Ruby resolves there.
module Integration
  module Completion
    module Probe
      class CompletionProbe < Godot::EditorPlugin
        tool

        PATH = "res://integration/completion/probe/signpost.rb".freeze
        BLANK_LINE = 5
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
          puts "engine class offered: #{collect_offers(code, '        Godot::Nod').include?('Node2D')}"
          puts "value type offered: #{collect_offers(code, '        Godot::Vec').include?('Vector2')}"
        end

        # The names the editor offers once `text` is typed on the blank line.
        def collect_offers(code, text)
          code.set_line(BLANK_LINE, text)
          code.set_caret_line(BLANK_LINE)
          code.set_caret_column(text.length)
          code.request_code_completion(true)
          code.get_code_completion_options.map { |option| option["display_text"] }
        end
      end
    end
  end
end
