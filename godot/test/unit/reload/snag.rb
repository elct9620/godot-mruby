module Unit
  module Reload
    # A node script whose file raises, after writing a method a node is
    # called with, until a test changes its source.
    class Snag < Godot::Node
      def answer
        "broken"
      end

      raise "Snag is not finished"
    end
  end
end
