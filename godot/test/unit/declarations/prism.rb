module Unit
  module Declarations
    # Exports resources a file holds, alone and inside containers, and
    # objects no file holds, alone and beside one a file holds.
    class Prism < Godot::Node2D
      SHEEN = Godot::ResourceLoader.load("res://test/unit/declarations/sheen.tres")

      export :sheen, SHEEN
      export :tint, Godot::Gradient.new
      export :palette, [SHEEN, { "glow" => SHEEN }]
      export :blend, [SHEEN, Godot::Gradient.new]
    end
  end
end
