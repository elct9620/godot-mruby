module Unit
  module Declarations
    # Exports one name with two values.
    class Differing < Godot::Node2D
      export :mode, :nearest
      export :mode, :first
    end
  end
end
