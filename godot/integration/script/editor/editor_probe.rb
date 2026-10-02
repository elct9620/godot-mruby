# In the editor, waits for the extension to run the knob's file at a frame,
# then prints the hint and the group its placeholder lists, and what a tool
# that does not parse kept of the scene; then widens the knob's bounds,
# reloads it, and prints the hint once the file ran again.
module Integration
  module Script
    module Editor
      class EditorProbe < Godot::Node
        tool

        # Frames for the extension to run a placeholder's file.
        WAIT = 3
        # Godot's PROPERTY_USAGE_GROUP, of PropertyUsageFlags.
        PROPERTY_USAGE_GROUP = 64

        # A person's editor is left alone: the probe changes a script's
        # source, which a save there would write to the file.
        def _ready
          @frames = 0
          @is_probing = Godot::Engine.is_editor_hint && Godot::DisplayServer.get_name == "headless"
        end

        def _process(_delta)
          return unless @is_probing

          @frames += 1
          if @frames == WAIT
            print_first_run
            widen
          elsif @frames == WAIT * 2
            puts "knob turn hint after reload: #{turn_hint}"
            @is_probing = false
          end
        end

        def print_first_run
          grouped = properties.any? do |property|
            property["name"] == "Turning" && property["usage"] == PROPERTY_USAGE_GROUP
          end
          puts "knob turn hint: #{turn_hint}, grouped: #{grouped}"
          puts "cracked kept: #{get_node('Cracked').get('kept')}"
        end

        def widen
          script = get_node("Knob").get_script
          script.source_code = script.source_code.sub("0..10", "0..20")
          script.reload
        end

        def properties
          get_node("Knob").get_property_list
        end

        def turn_hint
          properties.find { |property| property["name"] == "turn" }["hint_string"]
        end
      end
    end
  end
end
