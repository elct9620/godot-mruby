module Loader
  # Exports a container, which no two nodes share.
  class Stacking < Godot::Node2D
    export :rounds, [1, 2]
  end
end
