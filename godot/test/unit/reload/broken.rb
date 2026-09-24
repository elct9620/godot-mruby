module Unit
  module Reload
    # A node script whose file raises, after writing a method a node is
    # called with, until a test changes its source.
    class Broken < Godot::Node
      def answer
        "broken"
      end

      raise "Broken is not finished"
    end
  end
end
