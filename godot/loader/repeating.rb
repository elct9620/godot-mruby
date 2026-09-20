module Loader
  # Declares one signal twice, as it was declared the first time.
  class Repeating < Godot::Node
    signal :rung, :times
    signal :rung, :times
  end
end
