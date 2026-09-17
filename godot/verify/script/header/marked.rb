# A tool and abstract, which Godot is told without the file running.
module Verify
  module Script
    module Header
      class Marked < Godot::Node2D
        puts "marked.rb ran"

        tool
        abstract
      end
    end
  end
end
