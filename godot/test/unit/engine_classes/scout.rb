module Unit
  module EngineClasses
    # A node class extending Node2D, for a node of Node2D and one of a class
    # extending it, which has methods a Node2D does not.
    class Scout < Godot::Node2D
      def reaches_rect
        get_rect
        true
      rescue NoMethodError
        false
      end

      # What a call of such a method with too many arguments raises.
      def refusal
        get_rect(1)
      rescue ArgumentError => e
        e.message
      end
    end
  end
end
