# A node class with properties of its own: what it exports, the engine and
# the editor read and write by name.
class Turret < Godot::Node2D
  export :mode, :nearest
  export :range, 300.0
end
