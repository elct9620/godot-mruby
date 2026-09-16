# Defines no method Godot calls, so nothing ever runs this file, however the
# scene sets its node's properties.
module Verify
  module Script
    module Callbacks
      class Idle < Godot::Node
        puts "idle.rb ran"
      end
    end
  end
end
