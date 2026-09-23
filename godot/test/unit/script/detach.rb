module Unit
  module Script
    # A node class whose method takes its node's script away, then answers.
    class Detach < Godot::Node
      def detach
        set_script(nil)
        :detached
      end
    end
  end
end
