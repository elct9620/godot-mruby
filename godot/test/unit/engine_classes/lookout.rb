module Unit
  module EngineClasses
    # A node class whose own set_visible reaches the engine's through super,
    # counting the calls it ran.
    class Lookout < Godot::Node2D
      # Written with def so the script answers it before its file has run.
      def show_count
        @show_count
      end

      def set_visible(value)
        @show_count = (@show_count || 0) + 1
        super
      end
    end
  end
end
