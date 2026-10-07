# frozen_string_literal: true

module Godot
  # Plays the scene calling a value type's methods with fewer arguments than
  # they require on a release build. Part of Godot::Release.
  module Values
    SCENE = "res://integration/values/refused_method.tscn"
    REFUSALS = ["wrong number of arguments (given 1, expected 2)",
                "wrong number of arguments (given 1, expected 3)"].freeze

    module_function

    # @behavior RV-033
    def verify_release!(dir)
      refusals = Release.refusals(dir, SCENE)
      return if refusals == REFUSALS

      raise "The release build's value methods were not refused #{REFUSALS}, only #{refusals}"
    end
  end
end
