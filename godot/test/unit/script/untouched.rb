module Unit
  module Script
    # A node script under a test directory that the editor must never run,
    # by path or by name, so running says so.
    class Untouched < Godot::Node2D
      puts "a test directory's file ran in the editor"
    end
  end
end
