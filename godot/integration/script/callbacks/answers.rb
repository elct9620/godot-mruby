module Integration
  module Script
    module Callbacks
      class Answers < Godot::Node
        def self_loop
          looped = []
          looped << looped
        end
      end
    end
  end
end
