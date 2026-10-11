module Integration
  module Script
    module Callbacks
      class Answers < Godot::Node
        def self_loop
          looped = []
          looped << looped
        end

        def take(_value)
          puts "take was called"
        end
      end
    end
  end
end
