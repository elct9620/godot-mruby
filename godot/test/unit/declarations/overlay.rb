module Unit
  module Declarations
    # Exports a name its engine class already has a property of.
    class Overlay < Godot::Node2D
      export :position, 1
    end
  end
end
