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
    # A scene of nodes whose scripts define callbacks and methods Godot calls,
    # what they print, what they must never print, and the answers Godot gets.
    CALLBACKS_SCENE = "res://verify/script/callbacks/callbacks.tscn"
    READY_LINE = "ready.rb is ready"
    PROCESS_LINE = "process.rb was given a Float"
    BUILT_LINE = "built.rb built an object"
    FAILED_LINE = "ERROR: res://verify/script/callbacks/failing.rb: failing.rb cannot be built"
    FAILED_PROCESS_LINE = "failing.rb processed"
    IDLE_LINE = "idle.rb ran"
    NOTIFIED_LINE = "notified.rb was notified it is ready"
    ANSWER_LINES = ["ratio answered float 1.5", "count answered int 3"].freeze
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
      verify_callbacks!(project)
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

    # Runs the callbacks scene for a few frames and counts what its Ruby printed.
    def verify_callbacks!(project)
      output, status = Godot.run_scene(project, CALLBACKS_SCENE, "--quit-after", "5")
      raise "The callbacks scene did not run:\n#{output}" unless status.success?

      lines = output.lines.map(&:chomp)
      verify_callbacks_called!(lines, output)
      verify_objects_built!(lines, output)
      verify_answers_reached!(lines, output)
    end

    # @behavior RS-011 RS-012 RS-017
    def verify_callbacks_called!(lines, output)
      missing = [READY_LINE, PROCESS_LINE, NOTIFIED_LINE].reject { |line| lines.include?(line) }
      return if missing.empty?

      raise "The callbacks scene's callbacks were not called #{missing}:\n#{output}"
    end

    # @behavior RS-013 RS-014 RS-015 RS-016
    def verify_objects_built!(lines, output)
      counts = { BUILT_LINE => 2, FAILED_LINE => 1, FAILED_PROCESS_LINE => 0, IDLE_LINE => 0 }
      wrong = counts.reject { |line, count| lines.count(line) == count }
      return if wrong.empty?

      raise "The callbacks scene's objects were not built as they should be #{wrong.keys}:\n#{output}"
    end

    # @behavior RS-028
    def verify_answers_reached!(lines, output)
      missing = ANSWER_LINES.reject { |line| lines.include?(line) }
      return if missing.empty?

      raise "The callbacks scene's answers did not reach Godot as Ruby gave them #{missing}:\n#{output}"
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
