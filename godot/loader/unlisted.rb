module Loader
  # Exports a class no announcement names, since a library file's class is
  # in no editor's list.
  class Unlisted < Godot::Node2D
    export :potion, nil, type: Items::Potion
  end
end
