# frozen_string_literal: true

module Godot
  # Runs the integration-test project's scenes of node scripts and reads what
  # they print: which files a node refuses, what a node answers Godot before
  # its file runs, which callbacks a node's Ruby object is given, and what its
  # methods answer Godot. Part of Godot.verify!.
  module NodeScripts
    # A scene whose nodes take a library file, a node script extending a class
    # they are not, an abstract node script and a file extending a name no
    # file spells, and the errors refusing each.
    ATTACH_SCENE = "res://verify/script/attach/attach.tscn"
    REFUSALS = [
      "ERROR: res://verify/script/attach/library.rb defines no class extending an engine node class, " \
      "so it cannot be a node's script",
      "ERROR: res://verify/script/attach/planar.rb extends Node2D, so it cannot be the script of a Node",
      'ERROR: Node "Abstract" previously had a script, but that script is now abstract.',
      "ERROR: res://verify/script/attach/orphan.rb extends Missing, which no one file names, " \
      "so it cannot be a node's script"
    ].freeze

    module_function

    def verify!(project)
      verify_attach_refused!(project)
      Header.verify!(project)
      Callbacks.verify!(project)
      Exports.verify!(project)
    end

    # Runs the scene whose nodes take scripts they cannot, each refused in the log.
    # @behavior RS-006 RS-007 RS-020 RS-024
    def verify_attach_refused!(project)
      output, status = Godot.run_scene(project, ATTACH_SCENE, "--quit-after", "3")
      missing = REFUSALS.reject { |refusal| output.include?(refusal) }
      return if status.success? && missing.empty?

      raise "The attach scene's scripts were not refused as they should be #{missing}:\n#{output}"
    end
  end
end
