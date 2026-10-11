# Two nodes share this file, and each has its object built once however many
# callbacks it is given.
module Integration
  module Script
    module Callbacks
      class Brick < Godot::Node
        def initialize
          puts "brick.rb built an object"
        end

        def _ready
        end

        def _process(delta)
        end
      end
    end
  end
end
