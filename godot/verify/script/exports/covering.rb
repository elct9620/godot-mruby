module Verify
  module Script
    module Exports
      # Writes an instance variable named as a property only the node it is
      # attached to has, which stays the engine's to answer: the node is a
      # Sprite2D while this class extends Node2D.
      class Covering < Godot::Node2D
        def initialize
          @centered = :mine
        end

        def cover
          puts "covering.rb covers #{@centered}"
        end
      end
    end
  end
end
