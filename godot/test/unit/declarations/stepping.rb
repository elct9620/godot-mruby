module Unit
  module Declarations
    # Exports a value with a step and no range for it to step through.
    class Stepping < Godot::Node2D
      export :pace, 1, step: 2
    end
  end
end
