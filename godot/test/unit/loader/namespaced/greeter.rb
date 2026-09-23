module Unit
  module Loader
    # namespaced.rb has run before this file, so the namespace it reopens has
    # GREETING.
    module Namespaced
      class Greeter < Godot::Node
        def greeting
          GREETING
        end
      end
    end
  end
end
