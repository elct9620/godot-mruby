module Loader
  # Exports a node's class with a number for its value, which no node is.
  class Misvalued < Godot::Node2D
    export :spot, 3, type: Godot::Marker2D
  end
end
