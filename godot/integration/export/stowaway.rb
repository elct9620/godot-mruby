# Holds a test runner node and prints whether the game knows the runner's
# class, which the library an export template loads leaves out.
module Integration
  module Export
    class Stowaway < Godot::Node
      def _ready
        puts "runner class: #{Godot::ClassDB.class_exists('RubyTestRunner')}"
      end
    end
  end
end
