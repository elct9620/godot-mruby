# Calls an engine method's bind with fewer arguments than it requires and
# with more than it takes, which a game exported without the engine's debug
# checks hands the engine unchecked, then quits.
module Integration
  module EngineClasses
    class RefusedBind < Godot::Node
      def _ready
        [[], [:refused, 1, 2]].each do |args|
          set_meta(*args)
        rescue ArgumentError => e
          puts "refused: #{e.message}"
        end
        get_tree.quit
      end
    end
  end
end
