module Integration
  module Script
    module Callbacks
      class Fault < Godot::Node
        def initialize
          raise "fault.rb cannot be built"
        end

        def _process(delta)
          puts "fault.rb processed"
        end
      end
    end
  end
end
