# frozen_string_literal: true

module Godot
  # Plays the scene calling an engine method's bind with counts it does not
  # take on a release build. Part of Godot::Release.
  module EngineClasses
    SCENE = "res://integration/engine_classes/refused_bind.tscn"
    REFUSALS = ["wrong number of arguments (given 0, expected 2)",
                "wrong number of arguments (given 3, expected 2)"].freeze

    module_function

    # @behavior RG-037
    def verify_release!(dir)
      refusals = Release.refusals(dir, SCENE)
      return if refusals == REFUSALS

      raise "The release build's bind was not refused #{REFUSALS}, only #{refusals}"
    end
  end
end
