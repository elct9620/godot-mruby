# An editor plugin written in Ruby, which says when the editor adds it.
module Integration
  module Script
    module Plugin
      class HelloPlugin < Godot::EditorPlugin
        tool

        def _enter_tree
          puts "a Ruby editor plugin entered the tree"
        end
      end
    end
  end
end
