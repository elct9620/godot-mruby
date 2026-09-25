# A tool script with a syntax error, which the editor must not run as a tool
# but keep the values a scene wrote for, as it does for a GDScript that does
# not parse.
module Integration
  module Script
    module Editor
      class Cracked < Godot::Node2D
        tool

        def _ready
          puts "the cracked tool is ready in the editor"
        end

        def glow(
      end
    end
  end
end
