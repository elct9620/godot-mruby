# Declares a signal a scene connects, which Godot is told about without the
# file running.
module Verify
  module Script
    module Header
      class Siren < Godot::Node2D
        puts "siren.rb ran"

        signal :wailed, :times

        def wail
          emit_signal(:wailed, 2)
        end
      end
    end
  end
end
