module Unit
  module EngineClasses
    # A node class extending an abstract one, writing its abstract method.
    class Rabbit < Critter
      def hop(height)
        "hops #{height}"
      end
    end
  end
end
