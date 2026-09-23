# Names the constant of a file under a test directory, and prints whether
# Ruby reached it.
module Integration
  module Export
    class Seeker < Godot::Node
      def _ready
        puts "reached #{::Leftover}"
      rescue NameError => e
        puts "left out: #{e.message}"
      end
    end
  end
end
