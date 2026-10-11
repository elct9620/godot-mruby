module Unit
  module Script
    module Lineage
      # Defines the callback its subclasses inherit.
      class Enemy < Godot::Node2D
        attr_reader :ready_class_name

        def _ready
          @ready_class_name = self.class.to_s
        end
      end
    end
  end
end
