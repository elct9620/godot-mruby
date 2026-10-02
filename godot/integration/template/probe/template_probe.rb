# Creates a Ruby script from CharacterBody2D's Basic Movement template in the
# script dialog, under a namespace, as a person would from the editor, after
# printing the templates the dialog lists for Node2D. The dialog fills its
# template menu after it pops up, so each step waits a few frames.
module Integration
  module Template
    module Probe
      class TemplateProbe < Godot::EditorPlugin
        tool

        DIRECTORY = "res://enemies/ships".freeze
        PATH = "#{DIRECTORY}/hero_ship.rb".freeze
        TEMPLATE = "CharacterBody2D: Basic Movement".freeze
        OPEN_AT = 30
        LIST_AT = 40
        CREATE_AT = 50

        def _ready
          @frames = 0
        end

        def _process(_delta)
          @frames += 1
          case @frames
          when OPEN_AT then open
          when LIST_AT
            list
            @dialog.config("CharacterBody2D", PATH)
          when CREATE_AT
            choose(TEMPLATE)
            @dialog.get_ok_button.emit_signal(:pressed)
          end
        end

        def open
          Godot::DirAccess.make_dir_recursive_absolute(DIRECTORY)
          @dialog = Godot::ScriptCreateDialog.new
          Godot::EditorInterface.get_base_control.add_child(@dialog)
          @dialog.connect(:script_created, method(:report_created))
          choose("Ruby", menus.first)
          @dialog.config("Node2D", PATH)
          @dialog.popup_centered
        end

        def list
          each_item { |_menu, _index, text| puts "template listed: #{text}" }
        end

        def choose(text, only = nil)
          each_item(only) do |menu, index, item|
            next unless item == text

            menu.select(index)
            menu.emit_signal(:item_selected, index)
          end
        end

        def each_item(only = nil)
          (only ? [only] : menus).each do |menu|
            menu.item_count.times { |index| yield menu, index, menu.get_item_text(index) }
          end
        end

        def menus
          @dialog.find_children("*", "OptionButton", true, false)
        end

        def report_created(script)
          puts "template made: #{script.resource_path}"
        end
      end
    end
  end
end
