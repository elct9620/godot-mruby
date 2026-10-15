module Unit
  module Declarations
    # Declares one signal with two shapes, which the second declaration refuses.
    class Gong < Godot::Node
      signal :rung, times: Integer
      signal :rung, times: Integer, loudly: Godot::Bool
    end
  end
end
