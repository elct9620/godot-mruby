module Unit
  module Script
    # A node class whose initialize calls back into its own node.
    class Sprinter < Godot::Node
      attr_reader :last_notification

      def initialize
        notification(3)
      end

      def _notification(what)
        @last_notification = what
      end
    end
  end
end
