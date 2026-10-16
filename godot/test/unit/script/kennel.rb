module Unit
  module Script
    # A node class keeping the class of the hounds it houses inside it, each
    # exporting what the other does not.
    class Kennel < Godot::Node
      export :capacity, 3

      # A node class written inside another's, as a GDScript inner class is.
      class Hound < Godot::Node2D
        export :bark, "woof"
        signal :howled

        def fetch
          "stick"
        end
      end
    end
  end
end
