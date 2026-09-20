module Loader
  # Exports a name every Ruby object answers, which its accessors would take
  # away.
  class Obscuring < Godot::Node2D
    export :hash, 1
  end
end
