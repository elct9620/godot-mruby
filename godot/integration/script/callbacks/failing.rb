module Integration
  module Script
    module Callbacks
      class Failing < Godot::Node
        def initialize
          raise "failing.rb cannot be built"
        end

        def _process(delta)
          puts "failing.rb processed"
        end
      end
    end
  end
end
