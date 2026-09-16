# frozen_string_literal: true

module Godot
  # Runs the integration-test project's scenes of node scripts and reads what
  # they print: which files a node refuses, what a script answers before its
  # file runs, and which callbacks a node's Ruby object is given. Part of
  # Godot.verify!.
  module NodeScripts
    # A scene whose nodes take a library file and a node script extending a
    # class they are not, and the errors refusing each.
    ATTACH_SCENE = "res://verify/script/attach/attach.tscn"
    REFUSALS = [
      "ERROR: res://verify/script/attach/library.rb defines no class extending an engine node class, " \
      "so it cannot be a node's script",
      "ERROR: res://verify/script/attach/planar.rb extends Node2D, so it cannot be the script of a Node"
    ].freeze
    # A scene asking a node script what Godot asks before its file runs, the
    # answers it prints, and the line the file prints if it ever runs.
    HEADER_SCENE = "res://verify/script/header/header.tscn"
    BASE_TYPE_ANSWER = "instance base type: Node2D"
    METHOD_ANSWER = "has _ready: true"
    REPORTED_RAN = "reported.rb ran"
    # A scene of nodes whose scripts define callbacks, what they print, and
    # what they must never print.
    CALLBACKS_SCENE = "res://verify/script/callbacks/callbacks.tscn"
    READY_LINE = "ready.rb is ready"
    PROCESS_LINE = "process.rb was given a Float"
    BUILT_LINE = "built.rb built an object"
    FAILED_LINE = "ERROR: res://verify/script/callbacks/failing.rb: failing.rb cannot be built"
    FAILED_PROCESS_LINE = "failing.rb processed"
    IDLE_LINE = "idle.rb ran"
    NOTIFIED_LINE = "notified.rb was notified it is ready"

    module_function

    def verify!(project)
      verify_attach_refused!(project)
      verify_base_type_answered!(project)
      verify_method_answered!(project)
      verify_callbacks!(project)
    end

    # Runs the scene whose nodes take scripts they cannot, each refused in the log.
    # @behavior RS-006 RS-007
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

    def verify_answer!(project, answer)
      output, status = Godot.run_scene(project, HEADER_SCENE, "--quit-after", "3")
      lines = output.lines.map(&:chomp)
      return if status.success? && lines.include?(answer) && !lines.include?(REPORTED_RAN)

      raise "The header scene did not answer #{answer.inspect} without running its file:\n#{output}"
    end

    # Runs the callbacks scene for a few frames and counts what its Ruby printed.
    def verify_callbacks!(project)
      output, status = Godot.run_scene(project, CALLBACKS_SCENE, "--quit-after", "5")
      raise "The callbacks scene did not run:\n#{output}" unless status.success?

      lines = output.lines.map(&:chomp)
      verify_callbacks_called!(lines, output)
      verify_objects_built!(lines, output)
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
  end
end
