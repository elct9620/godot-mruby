module Unit
  module Declarations
    # Exports a string with a range, which only a number is read with.
    class Sign < Godot::Node2D
      export :label, "", range: 1..10
    end
  end
end
