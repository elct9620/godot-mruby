# A callback the debugger stops in: it counts on four lines, the first and
# third calling methods in another file, one frame below it.
module Integration
  module Debugger
    class Stepper < Godot::Node
      def _ready
        count = Tally.start
        count += 1
        count = Tally.bump(count)
        puts "stepped to #{count}"
      end
    end
  end
end
