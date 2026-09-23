# Two nodes share this file, and each has its object built once however many
# callbacks it is given.
module Integration
  module Script
    module Callbacks
      class Built < Godot::Node
        def initialize
          puts "built.rb built an object"
        end

        def _ready
        end

        def _process(delta)
        end
      end
    end
  end
end
