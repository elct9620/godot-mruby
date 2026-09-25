module Unit
  module Script
    # A node class exporting a property whose initialize raises, so a node
    # the engine makes for it never has an object.
    class Brittle < Godot::Node
      export :armor, 1

      def initialize
        raise "Brittle cannot be initialized"
      end
    end
  end
end
