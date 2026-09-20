module Loader
  # Reads what it was exported as it initializes, so the value is there
  # before anything else runs.
  class Priming < Godot::Node2D
    export :range, 300.0

    attr_reader :primed

    def initialize
      @primed = @range
    end
  end
end
