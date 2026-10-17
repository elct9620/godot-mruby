# Pushes an error and a warning from a method its callback calls, one frame
# below it.
module Integration
  module Report
    class Herald < Godot::Node
      def _ready
        proclaim
      end

      def proclaim
        Godot.push_error("herald's error")
        Godot.push_warning("herald's warning")
      end
    end
  end
end
