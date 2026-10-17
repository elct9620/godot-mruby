module Unit
  module EngineClasses
    # An abstract node class marking `icon`, a name Godot::Object answers on
    # the class, as a method the one in charm.rb leaves unwritten.
    class Trinket < Godot::Node
      abstract

      abstract :icon
    end
  end
end
