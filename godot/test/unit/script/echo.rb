module Unit
  module Script
    # A node class whose method calls back into its own node.
    class Echo < Godot::Node
      attr_reader :last_notification

      def _notification(what)
        @last_notification = what
      end

      def ping
        notification(7)
        :pinged
      end
    end
  end
end
