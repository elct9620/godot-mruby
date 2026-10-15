module Unit
  module Declarations
    # Exports a resource a file holds and one no file holds.
    class Prism < Godot::Node2D
      export :sheen, Godot::ResourceLoader.load("res://test/unit/declarations/sheen.tres")
      export :tint, Godot::Gradient.new
    end
  end
end
