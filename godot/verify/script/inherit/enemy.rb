# Defines the callback its subclasses inherit, and a method of a name the
# engine has none of, which is how a subclass's list is seen to carry it.
module Verify
  module Script
    module Inherit
      class Enemy < Godot::Node2D
        def _ready
          puts "#{self.class} is ready"
        end

        def charge; end
      end
    end
  end
end
