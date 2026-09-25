# A node script whose hint only running tells, and which uses a constant a
# test directory's file names, as the editor's realm runs it.
module Integration
  module Script
    module Editor
      class Knob < Godot::Node2D
        BOUNDS = 0..10
        REACHED = (Unit::Script::Untouched rescue nil)

        export_group "Turning"
        export :turn, 5, range: BOUNDS
      end
    end
  end
end
