module Unit
  module Declarations
    # Writes an instance variable its class never exported, which Godot still
    # reaches by name.
    class Pouch < Godot::Node2D
      def initialize
        @stash = 7
      end

      def stash
        @stash
      end
    end
  end
end
