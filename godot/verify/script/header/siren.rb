# Declares a signal a scene connects and the properties Godot reads, which
# it is told about without the file running. The value of a property built
# as the file runs is none of the header's, so it answers nothing until then.
module Verify
  module Script
    module Header
      class Siren < Godot::Node2D
        puts "siren.rb ran"

        signal :wailed, :times

        export :volume, 11
        export :heard_at, Godot::Vector2.new(1, 2)
      end
    end
  end
end
