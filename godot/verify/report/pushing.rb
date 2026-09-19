# Reports to the engine as its class is defined, through Godot's own
# error and warning functions.
module Verify
  module Report
    class Pushing < Godot::Node
      Godot.push_error("push_error from mruby")
      Godot.push_warning("push_warning from mruby")

      def _ready
      end
    end
  end
end
