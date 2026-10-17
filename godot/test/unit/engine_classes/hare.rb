module Unit
  module EngineClasses
    # A node class writing its abstract parent's method by calling the
    # parent's own.
    class Hare < Critter
      def hop(height)
        super
      end
    end
  end
end
