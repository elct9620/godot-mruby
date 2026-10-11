module Unit
  module Declarations
    # Exports a value with a keyword that names no hint.
    class Tag < Godot::Node2D
      export :pick, 1, slider: true
    end
  end
end
