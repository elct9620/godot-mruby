# Defines the callback its subclasses inherit.
module Verify
  module Script
    module Inherit
      class Enemy < Godot::Node2D
        def _ready
          puts "#{self.class} is ready"
        end
      end
    end
  end
end
