module Unit
  module Scene
    # A node class keeping every delta its _process and _physics_process are
    # given.
    class Ticker < Godot::Node
      def processed
        @processed ||= []
      end

      def physics_processed
        @physics_processed ||= []
      end

      def _process(delta)
        processed << delta
      end

      def _physics_process(delta)
        physics_processed << delta
      end
    end
  end
end
