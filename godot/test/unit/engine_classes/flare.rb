# Uses Godot::Marker3D, which no other file uses, then raises, so the engine
# class is made while this file runs and has to outlive what it takes away.
module Unit
  module EngineClasses
    class Flare < Godot::Marker3D
    end
  end

  raise "flare.rb fails after using an engine class"
end
