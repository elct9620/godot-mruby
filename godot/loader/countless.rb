module Loader
  # Exports a class that is neither a node nor a resource, so nothing in the
  # editor could choose a value for it.
  class Countless < Godot::Node2D
    export :counted, nil, type: Godot::RefCounted
  end
end
