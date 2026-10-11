module Unit
  module Declarations
    # Exports one property twice, as it was exported the first time.
    class Ledger < Godot::Node2D
      export :mode, :nearest
      export :mode, :nearest
    end
  end
end
