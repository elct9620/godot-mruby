module Unit
  module Script
    module Inherited
      # Defines the callback its subclasses inherit.
      class Enemy < Godot::Node2D
        attr_reader :readied

        def _ready
          @readied = self.class.to_s
        end
      end
    end
  end
end
