# Hands a node script's documentation to the editor's help, as the script
# editor does once the script is saved, opens the class's page and prints
# whether its class and method comments are shown.
module Integration
  module Documentation
    module Probe
      class DocumentationProbe < Godot::EditorPlugin
        tool

        PATH = "res://integration/documentation/probe/waymark.rb".freeze
        PAGE = "Waymark".freeze
        OPEN_AT = 30
        SHOW_AT = 40
        READ_AT = 50

        def _ready
          @frames = 0
        end

        def _process(_delta)
          @frames += 1
          if @frames == OPEN_AT
            open_page
          elsif @frames == SHOW_AT
            show_page
          elsif @frames == READ_AT
            read_page
          end
        end

        def open_page
          editor = Godot::EditorInterface.get_script_editor
          Godot::EditorInterface.set_main_screen_editor("Script")
          editor.update_docs_from_script(Godot::ResourceLoader.load(PATH))
          editor.goto_help("class_name:#{PAGE}")
        end

        # Makes the page the shown tab: the help draws a page only once it
        # is visible.
        def show_page
          page = find_page
          page.get_parent.current_tab = page.get_index if page
        end

        def read_page
          text = find_page&.find_children("*", "RichTextLabel", true, false)&.map(&:get_parsed_text)&.join || ""
          puts "class comment shown: #{text.include?('A waymark the probe documents.')}"
          puts "method comment shown: #{text.include?('Glows for the probe.')}"
        end

        def find_page
          Godot::EditorInterface.get_base_control
                                .find_children("*", "EditorHelp", true, false)
                                .find { |page| page.name.to_s == PAGE }
        end
      end
    end
  end
end
