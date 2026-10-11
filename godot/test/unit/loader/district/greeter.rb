module Unit
  module Loader
    # district.rb has run before this file, so the namespace it reopens has
    # GREETING.
    module District
      class Greeter < Godot::Node
        def greeting
          GREETING
        end
      end
    end
  end
end
