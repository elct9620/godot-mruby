# Prints as its class is defined, so each line appears once however many nodes
# share the file; _ready is what makes a node build its object and run it.
module Verify
  module Script
    class Prints < Godot::Node
      puts "puts from mruby"
      print "print from mruby", "\n"
      p :p_from_mruby

      def _ready
      end
    end
  end
end
