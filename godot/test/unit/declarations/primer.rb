module Unit
  module Declarations
    # Reads what it was exported as it initializes, so the value is there
    # before anything else runs.
    class Primer < Godot::Node2D
      export :range, 300.0

      attr_reader :initial_range

      def initialize
        @initial_range = @range
      end
    end
  end
end
