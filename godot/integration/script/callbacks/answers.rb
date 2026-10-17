# Answers GDScript with an Array holding itself, which cannot cross to the
# engine.
module Integration
  module Script
    module Callbacks
      class Answers < Godot::Node
        def make_self_loop
          looped = []
          looped << looped
        end
      end
    end
  end
end
