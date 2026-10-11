module Unit
  module Declarations
    # Exports one name with two values.
    class Lever < Godot::Node2D
      export :mode, :nearest
      export :mode, :first
    end
  end
end
