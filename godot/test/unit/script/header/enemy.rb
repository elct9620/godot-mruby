module Unit
  module Script
    module Header
      # Extends an engine node class and defines a method of a name the engine
      # has none of, which a subclass's list is seen to carry.
      class Enemy < Godot::Node2D
        def charge
        end
      end
    end
  end
end
