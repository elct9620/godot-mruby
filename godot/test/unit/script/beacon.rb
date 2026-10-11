module Unit
  module Script
    # A node class Ruby makes with the brightness it shines at.
    class Beacon < Godot::Node2D
      attr_reader :brightness, :last_notification

      def initialize(brightness)
        @brightness = brightness
      end

      def _notification(what)
        @last_notification = what
      end
    end
  end
end
