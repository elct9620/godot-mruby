module Integration
  module Script
    module Callbacks
      class Itself < Godot::Node
        def _ready
          puts "itself.rb is inside the tree: #{is_inside_tree}"
        end
      end
    end
  end
end
