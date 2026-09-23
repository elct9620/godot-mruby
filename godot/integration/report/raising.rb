# A callback that raises from a method it calls, one frame below it.
module Integration
  module Report
    class Raising < Godot::Node
      def _ready
        unlock
      end

      def unlock
        raise "wrong key"
      end
    end
  end
end
