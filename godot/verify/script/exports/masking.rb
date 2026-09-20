module Verify
  module Script
    module Exports
      # Writes an instance variable named as a property only the node it is
      # attached to has, which stays the engine's to answer.
      class Masking < Godot::Node2D
        def initialize
          @centered = :mine
        end

        def mark
          puts "masking.rb marks #{@centered}"
        end
      end
    end
  end
end
