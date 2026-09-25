module Unit
  module Declarations
    # Exports a value with `enum:` naming one name rather than a list of them.
    class Listless < Godot::Node2D
      export :pick, 1, enum: :one
    end
  end
end
