module Unit
  module Declarations
    # Exports a resource class with a resource of another class for its value.
    class Miscast < Godot::Node2D
      export :level, Godot::Resource.new, type: Godot::PackedScene
    end
  end
end
