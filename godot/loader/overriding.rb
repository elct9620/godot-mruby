module Loader
  # Exports a name its engine class already has a property of.
  class Overriding < Godot::Node2D
    export :position, 1
  end
end
