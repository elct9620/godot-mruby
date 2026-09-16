# Uses Godot::Marker3D, which no other file uses, then raises, so the engine
# class is made while this file runs and has to outlive what it takes away.
module Loader
  class EngineRaising < Godot::Marker3D
  end
end

raise "engine_raising.rb fails after using an engine class"
