module Unit
  module Values
    # Keeps what the engine last handed its method.
    class Keeper < Godot::Node
      attr_reader :kept

      def keep(value)
        @kept = value
      end
    end
  end
end
