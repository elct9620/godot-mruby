# A tool script, whose node the editor gives its callbacks as a game does.
module Integration
  module Script
    module Editor
      class Torch < Godot::Node2D
        tool

        def _ready
          puts "the torch is ready in the editor"
        end

        def _process(_delta)
          return if @lit

          @lit = true
          puts "the torch is processed in the editor"
        end
      end
    end
  end
end
