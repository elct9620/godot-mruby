# Asks the engine for a node it does not have, from a method its callback
# calls one frame below it, so the engine reports an error inside Ruby.
module Integration
  module Report
    class Wanderer < Godot::Node
      def _ready
        wander
      end

      def wander
        get_node("Nowhere")
      end
    end
  end
end
