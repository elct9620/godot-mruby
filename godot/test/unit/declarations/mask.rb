module Unit
  module Declarations
    # Writes an instance variable named as one of its engine class's
    # properties, which stays the engine's to answer.
    class Mask < Godot::Node2D
      def initialize
        @position = :mine
      end

      def own_position
        @position
      end
    end
  end
end
