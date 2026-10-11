module Unit
  module Script
    # A node carrying this file's own script, called back into before the
    # class below is defined.
    EARLY_NODE = Godot::Node.new
    EARLY_NODE.set_script(Godot::ResourceLoader.load("res://test/unit/script/forerunner.rb"))
    EARLY_NODE.notification(1)

    # The class of the node above.
    class Forerunner < Godot::Node
      def last_notification
        @last_notification
      end

      def _notification(what)
        @last_notification = what
      end
    end
  end
end
