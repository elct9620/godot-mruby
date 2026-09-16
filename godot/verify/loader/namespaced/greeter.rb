# namespaced.rb has run before this file, so the namespace it reopens has
# GREETING.
module Verify
  module Loader
    module Namespaced
      class Greeter < Godot::Node
        puts GREETING

        def _ready
        end
      end
    end
  end
end
