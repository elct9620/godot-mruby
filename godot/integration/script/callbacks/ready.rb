module Integration
  module Script
    module Callbacks
      class Ready < Godot::Node
        def _ready
          puts "ready.rb is ready"
        end
      end
    end
  end
end
