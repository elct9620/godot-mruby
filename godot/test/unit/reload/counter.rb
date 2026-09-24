module Unit
  module Reload
    # A node script counting what it is asked, whose source a test changes
    # while a node has it.
    class Counter < Godot::Node
      LIMIT = 3

      def bump
        @count = (@count || 0) + 1
      end

      def answer
        "before"
      end

      def dropped
        "dropped"
      end
    end
  end
end
