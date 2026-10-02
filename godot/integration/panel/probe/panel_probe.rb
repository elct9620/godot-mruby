# Runs one test directory from the test panel and prints what the panel
# lists, then activates the first row and prints the line the script editor
# opened.
module Integration
  module Panel
    module Probe
      class PanelProbe < Godot::EditorPlugin
        tool

        DIRECTORY = "res://integration/runner/failing/assertion".freeze
        PRESS_AT = 30
        LOOK_EVERY = 30

        def _ready
          @frames = 0
        end

        def _process(_delta)
          @frames += 1
          panel = (@panel ||= find_panel(get_tree.root))
          return unless panel

          if @frames == PRESS_AT
            press(panel)
          elsif @pressed && !@opened && (@frames % LOOK_EVERY).zero?
            look(panel)
          end
        end

        def press(panel)
          directories = panel.find_children("*", "OptionButton", true, false).first
          directories.item_count.times do |index|
            directories.select(index) if directories.get_item_text(index) == DIRECTORY
          end
          button_by_text(panel, "Run").emit_signal(:pressed)
          @pressed = true
        end

        def look(panel)
          tree = panel.find_children("*", "Tree", true, false).first
          root = tree.get_root
          return if Godot::EditorInterface.is_playing_scene || root.nil? || root.get_child_count.zero?

          print_rows(panel, root)
          root.get_child(0).select(0)
          tree.emit_signal(:item_activated)
          @opened = true
          print_opened
        end

        def print_rows(panel, root)
          puts "panel status: #{panel.find_children('*', 'Label', true, false).first.text}"
          root.get_children.each do |row|
            puts "panel row: #{row.get_text(0)} | #{row.get_text(1)} | #{row.get_text(2)}"
          end
        end

        def print_opened
          editor = Godot::EditorInterface.get_script_editor
          code = editor.get_current_editor.get_base_editor
          puts "panel opened: #{editor.get_current_script.resource_path}:#{code.get_caret_line + 1} " \
               "shown: #{editor.is_visible_in_tree}"
        end

        def button_by_text(panel, text)
          panel.find_children("*", "Button", true, false).find { |button| button.text == text }
        end

        def find_panel(node)
          return node if node.get_class == "RubyTestPanel"

          node.get_children(true).each do |child|
            found = find_panel(child)
            return found if found
          end
          nil
        end
      end
    end
  end
end
