# Calls a value type's method and its static method with fewer arguments
# than each requires, which a game exported without the engine's debug
# checks hands the engine unchecked, then quits.
module Integration
  module Values
    class RefusedMethod < Godot::Node
      def _ready
        [-> { Godot::Vector2.new(1, 2).lerp(Godot::Vector2.new(0, 0)) },
         -> { Godot::Color.from_hsv(0.5) }].each do |call|
          call.call
        rescue ArgumentError => e
          puts "refused: #{e.message}"
        end
        get_tree.quit
      end
    end
  end
end
