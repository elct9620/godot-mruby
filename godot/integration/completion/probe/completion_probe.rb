# Opens a node script in the script editor, as a person would, types a name
# on its blank line, asks for completion and prints whether the editor
# offers what Ruby resolves there; then looks names up as a Ctrl-click does
# and prints where the editor went.
module Integration
  module Completion
    module Probe
      class CompletionProbe < Godot::EditorPlugin
        tool

        PATH = "res://integration/completion/probe/signpost.rb".freeze
        MILESTONE_PATH = "res://integration/completion/probe/milestone.rb".freeze
        BLANK_LINE = 6
        POINT_LINE = 7
        MILESTONE_LINE = 4
        OPENING_FRAME = 30
        LOOK_FRAME = 40
        LOOKUP_FRAME = 50
        READING_FRAME = 60

        def _ready
          @frames = 0
        end

        def _process(_delta)
          @frames += 1
          if @frames == OPENING_FRAME
            Godot::EditorInterface.edit_script(Godot::ResourceLoader.load(PATH))
          elsif @frames == LOOK_FRAME
            look
          elsif @frames == LOOKUP_FRAME
            look_up
          elsif @frames == READING_FRAME
            read_lookups
          end
        end

        def look
          code = Godot::EditorInterface.get_script_editor.get_current_editor.get_base_editor
          puts "engine class offered: #{collect_offers(code, '        Godot::Nod').include?('Node2D')}"
          puts "value type offered: #{collect_offers(code, '        Godot::Vec').include?('Vector2')}"
          puts "own method offered: #{collect_offers(code, '        self.poi').include?('point')}"
          puts "export offered: #{collect_offers(code, '        self.wid').include?('width')}"
          puts "engine method offered: #{collect_offers(code, '        self.add_ch').include?('add_child(…)')}"
          puts "project constant offered: #{collect_offers(code, '        Batt').include?('Battle')}"
        end

        def look_up
          code = Godot::EditorInterface.get_script_editor.get_current_editor.get_base_editor
          look_up_word(code, "point")
          puts "method looked up: #{code.get_caret_line == POINT_LINE}"
          look_up_word(code, "add_child")
          look_up_word(code, "Milestone")
        end

        def read_lookups
          pages = Godot::EditorInterface.get_base_control.find_children("*", "EditorHelp", true, false)
          puts "engine member looked up: #{pages.any? { |page| page.name.to_s == 'Node' }}"
          editor = Godot::EditorInterface.get_script_editor
          opened = editor.get_current_script.resource_path == MILESTONE_PATH &&
                   editor.get_current_editor.get_base_editor.get_caret_line == MILESTONE_LINE
          puts "constant looked up: #{opened}"
        end

        # Types `word` on the blank line and looks it up as a Ctrl-click on it
        # does.
        def look_up_word(code, word)
          code.set_line(BLANK_LINE, "        #{word}")
          code.emit_signal(:symbol_lookup, word, BLANK_LINE, 10)
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
