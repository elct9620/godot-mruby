module Unit
  module Declarations
    # Declares an rpc with a mode `@rpc` does not take.
    class Stray < Godot::Node
      rpc :wander, mode: :everyone
    end
  end
end
