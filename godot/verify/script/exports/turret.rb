module Verify
  module Script
    module Exports
      # A turret whose scene sets its range before anything builds it; the
      # method the scene's GDScript calls is what builds it.
      class Turret < Godot::Node2D
        export :range, 300.0

        def aim
          puts "turret.rb aims at #{@range}"
        end
      end
    end
  end
end
