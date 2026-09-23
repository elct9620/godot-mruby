module Unit
  module Declarations
    # Writes the headings its properties are shown under, so the inspector
    # groups them as it groups a GDScript node's.
    class Panel < Godot::Node2D
      export_category "Panel"
      export_group "Targeting", "aim_"
      export :aim_mode, :nearest
      export_subgroup "Look"
      export :tint, 1
    end
  end
end
