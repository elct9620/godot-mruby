module Unit
  module EngineClasses
    # An abstract node class the ones in rabbit.rb and hare.rb extend, with
    # a method each of them writes.
    class Critter < Godot::Node
      abstract

      abstract def hop(height); end
    end
  end
end
