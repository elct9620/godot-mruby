module Unit
  module Values
    # Assigns members of its own properties' values the ways GDScript does.
    class Mover < Godot::Node2D
      def lift
        position.y = 5
      end

      def push
        self.position.x += 1
      end

      def slide
        transform.origin.x = 7
      end

      def place
        corner = Godot::Vector2.new(0, 0)
        position.x, corner.y = 3, 4
        corner
      end

      def nudge_once
        position.x ||= 9
        position.y &&= 6
      end

      def copy_moved
        moved = position
        moved.x = 99
        moved
      end

      def spot
        Godot::Vector2.new(1, 1)
      end

      def move_spot
        spot.x = 2
      end
    end
  end
end
