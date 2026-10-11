module Unit
  module Script
    # A node class whose object answers Godot's read of an instance variable
    # with a number rather than the array the extension reads it from.
    class Babbler < Godot::Node
      def greet = "hello"

      def __read_variable__(_name) = 1
    end
  end
end
