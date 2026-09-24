# A node script whose source a tool script changes in the editor to export
# another property, which its placeholder is then told of.
module Integration
  module Script
    module Exports
      class Plated < Godot::Node2D
        export :plates, 1
      end
    end
  end
end
