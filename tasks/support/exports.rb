# frozen_string_literal: true

module Godot
  # Runs the integration-test project's scene of a node script whose exported
  # property a scene sets, and reads what it prints: what the property answers
  # before the node has a Ruby object, what the object was given once it was
  # built, and what a node the scene set nothing on answers. Part of
  # NodeScripts.verify!.
  module Exports
    SCENE = "res://verify/script/exports/exports.tscn"
    # The scene sets the property before anything builds the node's object,
    # so the answer is the scene's; the object is given it once it is built,
    # and a node the scene set nothing on answers what its class exported.
    STAGED_LINE = "staged range: 500.0"
    AIMED_LINE = "turret.rb aims at 500.0"
    BUILT_LINE = "built range: 500.0"
    DEFAULT_LINE = "default range: 300.0"
    # The node is a Sprite2D while its script extends Node2D, so a property
    # only the node's own class has is still the engine's to answer: the
    # engine's own getter says so, and the variable the object wrote under
    # that name is untouched before and after.
    MARKED_LINE = "masking.rb marks mine"
    MARKS = 2
    CENTERED_LINE = "centered: false"

    module_function

    # @behavior RD-030 RD-031 RD-032 RD-034
    def verify!(project)
      output, status = Godot.run_scene(project, SCENE, "--quit-after", "3")
      lines = output.lines.map(&:chomp)
      expected = [STAGED_LINE, AIMED_LINE, BUILT_LINE, DEFAULT_LINE, CENTERED_LINE]
      missing = expected.reject { |line| lines.include?(line) }
      missing << MARKED_LINE unless lines.count(MARKED_LINE) == MARKS
      return if status.success? && missing.empty?

      raise "The exports scene did not answer its properties #{missing}:\n#{output}"
    end
  end
end
