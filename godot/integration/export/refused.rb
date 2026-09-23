# Adds a test runner node, which an exported game no longer ships in a scene,
# to the running game.
module Integration
  module Export
    class Refused < Godot::Node
      def _ready
        add_child(Godot::RubyTestRunner.new)
      end
    end
  end
end
