module Unit
  module Declarations
    # Exports the objects Godot fills in: a node of the scene, a resource of
    # the project, and a node of a class this project's own files define.
    class Aiming < Godot::Node2D
      export :spot, nil, type: Godot::Marker2D
      export :level, nil, type: Godot::PackedScene
      export :guard, nil, type: Turret
    end
  end
end
