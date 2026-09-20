module Loader
  # Exports one property twice, as it was exported the first time.
  class Restating < Godot::Node2D
    export :mode, :nearest
    export :mode, :nearest
  end
end
