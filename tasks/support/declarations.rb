# frozen_string_literal: true

module Godot
  # Runs the integration-test project's scene of a node script whose exported
  # property a scene sets, and reads what it prints: what the property answers
  # before the node has a Ruby object, what the object was given once it was
  # built, and what a node the scene set nothing on answers. Part of
  # Godot.verify!. On a release build, plays the scene of a node listing its
  # properties, which has to list its export under no category of its class.
  # Part of Godot::Release.
  module Declarations
    SCENE = "res://integration/declarations/exports/exports.tscn"
    # The scene sets the property before anything builds the node's object,
    # so the answer is the scene's; the object is given it once it is built,
    # and a node the scene set nothing on answers what its class exported.
    SCENE_RANGE_LINE = "staged range: 500.0"
    AIM_LINE = "preset.rb aims at 500.0"
    OBJECT_RANGE_LINE = "built range: 500.0"
    DEFAULT_LINE = "default range: 300.0"
    RELEASE_SCENE = "res://integration/declarations/release/ledger.tscn"
    EXPORT_LINE = "export: balance"
    CATEGORY_LINE = "category: ledger.rb"

    module_function

    # @behavior RD-030 RD-031 RD-032
    def verify!(project)
      output, status = Godot.run_scene(project, SCENE, "--quit-after", "3")
      lines = output.lines.map(&:chomp)
      missing = [SCENE_RANGE_LINE, AIM_LINE, OBJECT_RANGE_LINE, DEFAULT_LINE].reject { |line| lines.include?(line) }
      return if status.success? && missing.empty?

      raise "The exports scene did not answer its properties #{missing}:\n#{output}"
    end

    # @behavior RD-083
    def verify_release!(dir)
      output, status = Release.play(dir, RELEASE_SCENE)
      lines = output.lines.map(&:chomp)
      return if status.success? && lines.include?(EXPORT_LINE) && !lines.include?(CATEGORY_LINE)

      raise "The release build did not list the ledger's export under no category of its class:\n#{output}"
    end
  end
end
