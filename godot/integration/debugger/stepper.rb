# A callback the debugger stops in: it counts on three lines, the second
# calling a method one frame below it.
module Integration
  module Debugger
    class Stepper < Godot::Node
      def _ready
        count = 1
        count = bump(count)
        puts "stepped to #{count}"
      end

      def bump(count)
        count + 1
      end
    end
  end
end
