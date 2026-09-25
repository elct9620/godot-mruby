# frozen_string_literal: true

require_relative "script/announcement"
require_relative "script/callbacks"
require_relative "script/editor"
require_relative "script/header"

module Godot
  # Runs the integration-test project's scenes of node scripts and reads what
  # they print: that a file runs once however many nodes it is attached to,
  # which files a node refuses, and, through the modules under Script, what a
  # node answers Godot before its file runs, which callbacks its Ruby object
  # is given, what the editor makes of it and how it is announced. Part of
  # Godot.verify!.
  module Script
    PRINTS_SCENE = "res://integration/script/prints.tscn"
    # What the prints scene's Ruby prints. Each line must appear exactly once:
    # the scene attaches the same file to two nodes, and a file runs once.
    PRINTED_LINES = ["puts from mruby", "print from mruby", ":p_from_mruby"].freeze
    # A scene whose nodes take a library file, a node script extending a class
    # they are not, an abstract node script and a file extending a name no
    # file spells, and the errors refusing each.
    ATTACH_SCENE = "res://integration/script/attach/attach.tscn"
    REFUSALS = [
      "ERROR: res://integration/script/attach/library.rb defines no class extending an engine node class, " \
      "so it cannot be a node's script",
      "ERROR: res://integration/script/attach/planar.rb extends Node2D, so it cannot be the script of a Node",
      'ERROR: Node "Abstract" previously had a class of type "Abstract", but that class is now abstract.',
      "ERROR: res://integration/script/attach/orphan.rb extends Missing, which no one file names, " \
      "so it cannot be a node's script"
    ].freeze

    module_function

    def verify!(project)
      verify_scripts_run!(project)
      verify_attach_refused!(project)
      Header.verify!(project)
      Callbacks.verify!(project)
      Editor.verify!(project)
      Announcement.verify!(project)
    end

    # Runs the prints scene for a few frames and counts the lines its Ruby
    # printed; --quit-after ends the run even when nothing does.
    # @behavior RS-001 RS-002 RS-003 RS-004
    def verify_scripts_run!(project)
      output, status = Godot.run_scene(project, PRINTS_SCENE, "--quit-after", "3")
      lines = output.lines.map(&:chomp)
      counts = PRINTED_LINES.to_h { |line| [line, lines.count(line)] }
      return if status.success? && output.lines.grep(FAILED).empty? && counts.values.all?(1)

      raise "The prints scene's Ruby did not print each line once #{counts}:\n#{output}"
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
