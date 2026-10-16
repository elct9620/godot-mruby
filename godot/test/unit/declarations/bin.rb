module Unit
  module Declarations
    # Exports a typed array declared with a number, which is no array.
    class Bin < Godot::Node
      export :things, 0, type: [Integer]
    end
  end
end
