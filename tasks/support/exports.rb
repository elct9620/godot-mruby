# frozen_string_literal: true

module Godot
  # Runs the integration-test project's scene of a node script whose exported
  # property a scene sets, and reads what it prints: what the property answers
  # before the node has a Ruby object, what the object was given once it was
  # built, and what a node the scene set nothing on answers. Part of
  # NodeScripts.verify!.
  module Exports
    SCENE = "res://integration/declarations/exports/exports.tscn"
    # The scene sets the property before anything builds the node's object,
    # so the answer is the scene's; the object is given it once it is built,
    # and a node the scene set nothing on answers what its class exported.
    STAGED_LINE = "staged range: 500.0"
    AIMED_LINE = "preset.rb aims at 500.0"
    BUILT_LINE = "built range: 500.0"
    DEFAULT_LINE = "default range: 300.0"

    module_function

    # @behavior RD-030 RD-031 RD-032
    def verify!(project)
      output, status = Godot.run_scene(project, SCENE, "--quit-after", "3")
      lines = output.lines.map(&:chomp)
      missing = [STAGED_LINE, AIMED_LINE, BUILT_LINE, DEFAULT_LINE].reject { |line| lines.include?(line) }
      return if status.success? && missing.empty?

      raise "The exports scene did not answer its properties #{missing}:\n#{output}"
    end
  end
end
