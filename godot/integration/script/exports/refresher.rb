# In the editor, changes the Ruby node script's source and reloads it, then
# prints whether the node's placeholder lists the property it now exports;
# its value would come from the script whether or not the placeholder was told.
module Integration
  module Script
    module Exports
      class Refresher < Godot::Node
        tool

        REVISION = <<~RUBY.freeze
          module Integration
            module Script
              module Exports
                class Plate < Godot::Node2D
                  export :plates, 1
                  export :armor, 7
                end
              end
            end
          end
        RUBY

        # A person's editor is left alone: the probe changes a script's
        # source, which a save there would write to the file.
        def _ready
          return unless Godot::Engine.is_editor_hint && Godot::DisplayServer.get_name == "headless"

          plate = get_node("Plate")
          script = plate.get_script
          script.source_code = REVISION
          script.reload
          names = plate.get_property_list.map { |property| property["name"] }
          puts "armor listed: #{names.include?('armor')}"
        end
      end
    end
  end
end
