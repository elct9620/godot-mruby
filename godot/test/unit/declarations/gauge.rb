module Unit
  module Declarations
    # Exports a property for each hint a class can declare, so what the editor
    # is told to show reaches Godot as GDScript's `@export_*` annotations tell
    # it.
    class Gauge < Godot::Node2D
      export :angle, 0.0, range: 0.0..360.0
      export :speed, 1, range: 1..10, step: 2
      export :aim, :nearest, enum: [:nearest, :strongest]
      export :terrain, 0, flags: [:water, :fire]
      export :sound, "", file: "*.ogg"
      export :folder, "", dir: true
      export :notes, "", multiline: true
      export :title, "", placeholder: "Name"
    end
  end
end
