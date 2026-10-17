module Unit
  module Values
    # Holds a spot, counting each time it is read.
    class Ledger
      attr_reader :reads
      attr_writer :spot

      def initialize
        @reads = 0
        @spot = Godot::Vector2.new(1, 1)
      end

      def spot
        @reads += 1
        @spot
      end
    end
  end
end
