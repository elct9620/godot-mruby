module Loader
  # Reads a property its own way, which stands in place of the reader its
  # export would define.
  class Reading < Godot::Node2D
    def range
      @range * 2
    end

    export :range, 300.0
  end
end
