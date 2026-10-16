# frozen_string_literal: true

require_relative "script/announcement"
require_relative "script/callbacks"
require_relative "script/editor"
require_relative "script/header"
require_relative "script/singleton_node"

module Godot
  # Runs the integration-test project's scenes of node scripts and reads what
  # they print: that a file runs once however many nodes it is attached to,
  # which files a node refuses, and, through the modules under Script, what a
  # node answers Godot before its file runs, which callbacks its Ruby object
  # is given, what the editor makes of it and how it is announced. Part of
  # Godot.verify!.
  module Script
    PRINTER_SCENE = "res://integration/script/printer.tscn"
    # What the prints scene's Ruby prints. Each line must appear exactly once:
    # the scene attaches the same file to two nodes, and a file runs once.
    PRINTED_LINES = ["puts from mruby", "print from mruby", ":p_from_mruby"].freeze
    # A scene whose nodes take a library file, a node script extending a class
    # they are not, an abstract node script and a file extending a name no
    # file spells, and the errors refusing each.
    ATTACHMENT_SCENE = "res://integration/script/attachment/attachment.tscn"
    REFUSALS = [
      "ERROR: res://integration/script/attachment/library.rb defines no class extending an engine class, " \
      "so it cannot be the script of a Node",
      "ERROR: res://integration/script/attachment/plane.rb extends Node2D, so it cannot be the script of a Node",
      'ERROR: Node "Blueprint" previously had a class of type "Blueprint", but that class is now abstract.',
      "ERROR: res://integration/script/attachment/orphan.rb extends Missing, which no one file names, " \
      "so it cannot be the script of a Node"
    ].freeze

    module_function

    def verify!(project)
      verify_scripts_run!(project)
      verify_attach_refused!(project)
      Header.verify!(project)
      Callbacks.verify!(project)
      Editor.verify!(project)
      Announcement.verify!(project)
      SingletonNode.verify!(project)
    end

    # Runs the prints scene for a few frames and counts the lines its Ruby
    # printed; --quit-after ends the run even when nothing does.
    # @behavior RS-001 RS-002 RS-003 RS-004
    def verify_scripts_run!(project)
      output, status = Godot.run_scene(project, PRINTER_SCENE, "--quit-after", "3")
      lines = output.lines.map(&:chomp)
      counts = PRINTED_LINES.to_h { |line| [line, lines.count(line)] }
      return if status.success? && output.lines.grep(ERROR_LINE).empty? && counts.values.all?(1)

      raise "The prints scene's Ruby did not print each line once #{counts}:\n#{output}"
    end

    # Runs the scene whose nodes take scripts they cannot, each refused in the log.
    # @behavior RS-006 RS-007 RS-020 RS-024
    def verify_attach_refused!(project)
      output, status = Godot.run_scene(project, ATTACHMENT_SCENE, "--quit-after", "3")
      missing = REFUSALS.reject { |refusal| output.include?(refusal) }
      return if status.success? && missing.empty?

      raise "The attach scene's scripts were not refused as they should be #{missing}:\n#{output}"
    end
  end
end
