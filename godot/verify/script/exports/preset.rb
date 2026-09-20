module Verify
  module Script
    module Exports
      # A class whose exported value a scene presets before anything builds
      # its Ruby object; the method the scene's GDScript calls is what
      # builds it.
      class Preset < Godot::Node2D
        export :range, 300.0

        def aim
          puts "preset.rb aims at #{@range}"
        end
      end
    end
  end
end
