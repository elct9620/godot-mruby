# Prints if it ever runs, which asking about it must not make it do.
module Verify
  module Script
    module Header
      class Reported < Godot::Node2D
        puts "reported.rb ran"

        def _ready
        end
      end
    end
  end
end
