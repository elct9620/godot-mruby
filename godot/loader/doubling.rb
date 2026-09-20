module Loader
  # Exports one value with two hints, where a property carries one.
  class Doubling < Godot::Node2D
    export :pick, 1, range: 1..10, enum: [:one, :two]
  end
end
