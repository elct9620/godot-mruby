# Prints as its class is defined, so each line appears once however many nodes
# share the file.
module Verify
  module Script
    class Prints < Godot::Node
      puts "puts from mruby"
      print "print from mruby", "\n"
      p :p_from_mruby
    end
  end
end
