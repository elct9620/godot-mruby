# frozen_string_literal: true

require "open3"

module Godot
  # Opens a scene in a headless editor and reads what it printed: a node whose
  # script is a node script is given Godot's placeholder, so what the class
  # declares reaches the node while no Ruby runs. Part of Godot.verify!.
  module Editor
    SCENE = "res://integration/script/header/header.tscn"
    # The line the editor prints as it takes the scene in, which is what says
    # the run reached it; the editor scans the whole project first, so the run
    # is given frames enough to get there and quits by itself. A run told to
    # quit sooner ends during that scan and opens no scene at all.
    REACHED = "Loading resource: #{SCENE}".freeze
    FRAMES = "3000"
    # What Godot says of a connection to a signal the node has not got, which
    # is every scene connection while a node script has no script instance.
    REFUSED = "Attempt to connect nonexistent signal"
    DECLARED = "wailed"

    module_function

    # @behavior RS-045
    def verify!(project)
      output = open_scene(project)
      raise "The editor did not reach #{SCENE}:\n#{output}" unless output.include?(REACHED)

      refusals = output.lines.grep(/#{Regexp.escape(REFUSED)} '#{DECLARED}'/)
      return if refusals.empty?

      raise "The editor refused a connection a node script declares:\n#{refusals.join}"
    end

    def open_scene(project)
      output, status = Open3.capture2e(EXECUTABLE, "--headless", "--editor", "--verbose",
                                       "--path", project, SCENE, "--quit-after", FRAMES)
      raise "The editor did not run:\n#{output}" unless status.success?

      output
    end
  end
end
