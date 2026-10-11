module Unit
  module Scene
    # A node class keeping every delta its _process and _physics_process are
    # given.
    class Ticker < Godot::Node
      def deltas
        @deltas ||= []
      end

      def physics_deltas
        @physics_deltas ||= []
      end

      def _process(delta)
        deltas << delta
      end

      def _physics_process(delta)
        physics_deltas << delta
      end
    end
  end
end
