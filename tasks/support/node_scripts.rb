# frozen_string_literal: true

module Godot
  # Runs the integration-test project's scenes of node scripts and reads what
  # they print: which files a node refuses, what a script answers before its
  # file runs, which callbacks a node's Ruby object is given, and what its
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
    # A scene asking a node script what Godot asks before its file runs, the
    # answers it prints, and the line the file prints if it ever runs.
    HEADER_SCENE = "res://verify/script/header/header.tscn"
    BASE_TYPE_ANSWER = "instance base type: Node2D"
    METHOD_ANSWER = "has _ready: true"
    TOOL_ANSWER = "is tool: true"
    ABSTRACT_ANSWER = "is abstract: true"
    INHERITED_BASE_TYPE_ANSWER = "inherited base type: Node2D"
    BASE_SCRIPT_ANSWER = "base script: res://verify/script/inherit/enemy.rb"
    RAN_LINES = ["reported.rb ran", "marked.rb ran"].freeze
    # A scene whose node's script inherits its only callback from another
    # file, and what that callback prints.
    INHERIT_SCENE = "res://verify/script/inherit/inherit.tscn"
    INHERITED_READY_LINE = "Verify::Script::Inherit::Boss is ready"

    module_function

    def verify!(project)
      verify_attach_refused!(project)
      verify_base_type_answered!(project)
      verify_method_answered!(project)
      verify_tool_answered!(project)
      verify_abstract_answered!(project)
      verify_inherited_base_type_answered!(project)
      verify_base_script_answered!(project)
      Callbacks.verify!(project)
      verify_inherited_callback!(project)
    end

    # Runs the scene whose nodes take scripts they cannot, each refused in the log.
    # @behavior RS-006 RS-007 RS-020 RS-024
    def verify_attach_refused!(project)
      output, status = Godot.run_scene(project, ATTACH_SCENE, "--quit-after", "3")
      missing = REFUSALS.reject { |refusal| output.include?(refusal) }
      return if status.success? && missing.empty?

      raise "The attach scene's scripts were not refused as they should be #{missing}:\n#{output}"
    end

    # Runs the scene that asks a node script what Godot asks, whose answers have
    # to come from its source while its file never runs.
    # @behavior RS-008
    def verify_base_type_answered!(project)
      verify_answer!(project, BASE_TYPE_ANSWER)
    end

    # @behavior RS-009
    def verify_method_answered!(project)
      verify_answer!(project, METHOD_ANSWER)
    end

    # @behavior RS-018
    def verify_tool_answered!(project)
      verify_answer!(project, TOOL_ANSWER)
    end

    # @behavior RS-019
    def verify_abstract_answered!(project)
      verify_answer!(project, ABSTRACT_ANSWER)
    end

    # @behavior RS-021
    def verify_inherited_base_type_answered!(project)
      verify_answer!(project, INHERITED_BASE_TYPE_ANSWER)
    end

    # @behavior RS-022
    def verify_base_script_answered!(project)
      verify_answer!(project, BASE_SCRIPT_ANSWER)
    end

    def verify_answer!(project, answer)
      output, status = Godot.run_scene(project, HEADER_SCENE, "--quit-after", "3")
      lines = output.lines.map(&:chomp)
      return if status.success? && lines.include?(answer) && !lines.intersect?(RAN_LINES)

      raise "The header scene did not answer #{answer.inspect} without running its file:\n#{output}"
    end

    # Runs the scene whose node's only callback is inherited from another file.
    # @behavior RS-023
    def verify_inherited_callback!(project)
      output, status = Godot.run_scene(project, INHERIT_SCENE, "--quit-after", "3")
      return if status.success? && output.lines.map(&:chomp).include?(INHERITED_READY_LINE)

      raise "The inherit scene's node was not given its inherited _ready:\n#{output}"
    end
  end
end
