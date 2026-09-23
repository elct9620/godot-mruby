module Unit
  module Script
    # A node carrying this file's own script, called back into before the
    # class below is defined.
    EARLY_NODE = Godot::Node.new
    EARLY_NODE.set_script(Godot::ResourceLoader.load("res://test/unit/script/early.rb"))
    EARLY_NODE.notification(1)

    # The class of the node above.
    class Early < Godot::Node
      def notified
        @notified
      end

      def _notification(what)
        @notified = what
      end
    end
  end
end
