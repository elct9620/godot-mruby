module Unit
  module Script
    # An object of no engine class but Object, which Ruby frees itself.
    class Token < Godot::Object
      def label
        "token"
      end
    end
  end
end
