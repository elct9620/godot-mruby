module Unit
  module Script
    # A node script telling Godot what its speed reverts to.
    class Spring < Godot::Node
      def _property_can_revert(name)
        name == :speed
      end

      def _property_get_revert(name)
        12 if name == :speed
      end
    end
  end
end
