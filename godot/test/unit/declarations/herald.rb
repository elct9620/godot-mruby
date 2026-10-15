module Unit
  module Declarations
    # A node class whose signal carries a value of each kind a parameter is
    # typed by: a Ruby class, a bool, a value type, an engine class, an
    # announced node script's class and one announced by no name, and any
    # value.
    class Herald < Godot::Node
      signal :announced, count: Integer, loud: Godot::Bool, at: Godot::Vector2, spot: Godot::Node2D,
                         foe: ::Enemy, bell: Bell, payload: Godot::Variant
    end
  end
end
