module Unit
  module Declarations
    # Exports a value with a range that leaves out its end, which names no
    # highest value.
    class Endless < Godot::Node2D
      export :reach, 1, range: 1...9
    end
  end
end
