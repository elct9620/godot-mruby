module Unit
  module Script
    # A node class whose initialize calls back into its own node.
    class Eager < Godot::Node
      attr_reader :notified

      def initialize
        notification(3)
      end

      def _notification(what)
        @notified = what
      end
    end
  end
end
