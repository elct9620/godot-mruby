# Names the constant of a file under a test directory, and prints whether
# Ruby reached it.
module Verify
  module Export
    class Seeker < Godot::Node
      def _ready
        puts "reached #{::Test::Leftover}"
      rescue NameError => e
        puts "left out: #{e.message}"
      end
    end
  end
end
