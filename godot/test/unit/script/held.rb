module Unit
  module Script
    # A node script whose source a test replaces in Godot before any node
    # runs it.
    class Held < Godot::Node
      def source
        "the file"
      end
    end
  end
end
