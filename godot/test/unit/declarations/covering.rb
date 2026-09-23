module Unit
  module Declarations
    # Writes an instance variable named as a property only the node it is
    # attached to has: a test attaches it to a Sprite2D, while this class
    # extends Node2D.
    class Covering < Godot::Node2D
      def initialize
        @centered = "mine"
      end

      def covered
        @centered
      end
    end
  end
end
