module Unit
  module EngineClasses
    # A node class whose own set_visible reaches the engine's through super,
    # counting the calls it ran.
    class Lookout < Godot::Node2D
      attr_reader :shown

      def set_visible(value)
        @shown = (@shown || 0) + 1
        super
      end
    end
  end
end
