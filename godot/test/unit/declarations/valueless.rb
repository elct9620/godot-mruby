module Unit
  module Declarations
    # Exports a property with no value, which names no type.
    class Valueless < Godot::Node2D
      export :mode, nil
    end
  end
end
