module Unit
  module EngineClasses
    # An abstract node class with a class method and an instance method of
    # one name, marking the instance method abstract.
    class Codex < Godot::Node
      abstract

      def self.sum
        0
      end

      def sum
        1
      end
      abstract :sum
    end
  end
end
