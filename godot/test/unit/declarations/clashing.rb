module Unit
  module Declarations
    # Declares one signal with two shapes, which the second declaration refuses.
    class Clashing < Godot::Node
      signal :rung, :times
      signal :rung, :times, :loudly
    end
  end
end
