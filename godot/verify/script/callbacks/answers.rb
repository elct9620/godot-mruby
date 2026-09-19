module Verify
  module Script
    module Callbacks
      class Answers < Godot::Node
        def ratio
          1.5
        end

        def count
          3
        end

        def looped
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
