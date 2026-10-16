# The singleton node the check's copy of the project lists: says where it
# stands once ready, and answers the scene's GDScript, which reaches it by name.
module Integration
  module Script
    module SingletonNode
      class Steward < Godot::Node
        def _ready
          puts "steward ready at #{get_path}"
        end

        def greeting
          "welcome"
        end
      end
    end
  end
end
