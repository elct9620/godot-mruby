module Unit
  module Declarations
    # Declares one signal twice, as it was declared the first time.
    class Repeater < Godot::Node
      signal :rung, :times
      signal :rung, :times
    end
  end
end
