module Unit
  module Script
    # A node class whose initialize raises, keeping the object it was
    # initializing where a test can reach it.
    class Glitch < Godot::Node
      class << self
        attr_accessor :last
      end

      def initialize
        Glitch.last = self
        raise "Glitch cannot be initialized"
      end
    end
  end
end
