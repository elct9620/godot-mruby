# A node script whose signal carries a value of each kind a parameter is
# typed by, which the editor's Signals dock lists.
module Integration
  module Script
    module Signals
      class Beacon < Godot::Node2D
        signal :flared, count: Integer, lit: Godot::Bool, at: Godot::Vector2, spot: Godot::Node2D, foe: Enemy,
                        payload: Godot::Variant
      end
    end
  end
end
