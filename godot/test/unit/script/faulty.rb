module Unit
  module Script
    # A node class whose initialize raises, keeping the object it was
    # initializing where a test can reach it.
    class Faulty < Godot::Node
      class << self
        attr_accessor :last
      end

      def initialize
        Faulty.last = self
        raise "Faulty cannot be initialized"
      end
    end
  end
end
