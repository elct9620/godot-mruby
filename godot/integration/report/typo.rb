# The compiler stops at the stray end below, so this file never runs.
module Integration
  module Report
    class Typo < Godot::Node
      puts "typo.rb ran"

      def _ready
      end
    end
  end
end
end
