module Unit
  module Declarations
    # Writes an instance variable its class never exported, which Godot still
    # reaches by name.
    class Holding < Godot::Node2D
      def initialize
        @kept = 7
      end

      def kept
        @kept
      end
    end
  end
end
