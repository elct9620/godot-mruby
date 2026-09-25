# frozen_string_literal: true

require "fileutils"
require "open3"

module Godot
  module Script
    # Opens scenes in a headless editor and reads what it printed: a node whose
    # script is a node script is given Godot's placeholder, told what the
    # class declares before its file runs, as the editor's realm runs it, and
    # once its source changes; a tool's node is given its callbacks, and no
    # file under a test directory runs. Part of Script.verify!.
    module Editor
      SCENE = "res://integration/script/header/header.tscn"
      # A scene whose tool script changes a node script's source, reloads it,
      # and prints whether the node's placeholder lists the property the change
      # exports.
      EXPORTS_SCENE = "res://integration/script/exports/exports.tscn"
      EXPORTED = "armor listed: true"
      # A scene whose tool script prints what the extension told a node's
      # placeholder once the editor's realm ran its file, beside a node whose
      # script, under a test directory, the editor must never run.
      EDITOR_SCENE = "res://integration/script/editor/editor.tscn"
      RAN = "knob turn hint: 0,10, grouped: true"
      RAN_AGAIN = "knob turn hint after reload: 0,20"
      UNTOUCHED = "a test directory's file ran in the editor"
      KEPT = "cracked kept: 7"
      # What the scene's tool script prints as its node is given each callback.
      TOOL_CALLBACKS = ["the torch is ready in the editor", "the torch is processed in the editor"].freeze
      # The line the editor prints as it takes the scene in, which is what says
      # the run reached it; the editor scans the whole project first, so the run
      # is given frames enough to get there and quits by itself. A run told to
      # quit sooner ends during that scan and opens no scene at all.
      REACHED = "Loading resource: %s"
      # Where the editor keeps the scenes it has open, and writes them as it
      # quits, so the next editor opens them again. A scene opened to probe
      # it is put back out of it, so no later run, nor the person using the
      # editor, is left with it open.
      LAYOUT = ".godot/editor/editor_layout.cfg"
      FRAMES = "3000"
      # What Godot says of a connection to a signal the node has not got, which
      # is every scene connection while a node script has no script instance.
      REFUSED = "Attempt to connect nonexistent signal"
      DECLARED = "wailed"

      module_function

      def verify!(project)
        verify_connected!(project)
        verify_exported!(project)
        output = open_scene(project, EDITOR_SCENE)
        verify_ran!(output)
        verify_ran_again!(output)
        verify_left_out!(output)
        verify_tool_called!(output)
        verify_kept!(output)
      end

      # @behavior RS-045
      def verify_connected!(project)
        output = open_scene(project, SCENE)
        refusals = output.lines.grep(/#{Regexp.escape(REFUSED)} '#{DECLARED}'/)
        return if refusals.empty?

        raise "The editor refused a connection a node script declares:\n#{refusals.join}"
      end

      # @behavior RS-048
      def verify_exported!(project)
        output = open_scene(project, EXPORTS_SCENE)
        return if output.lines.map(&:chomp).include?(EXPORTED)

        raise "The editor's placeholder was not given what a reloaded node script exports:\n#{output}"
      end

      # @behavior RS-056
      def verify_ran!(output)
        return if output.lines.map(&:chomp).include?(RAN)

        raise "The editor's placeholder was not given what a node script declared as it ran:\n#{output}"
      end

      # @behavior RS-058
      def verify_ran_again!(output)
        return if output.lines.map(&:chomp).include?(RAN_AGAIN)

        raise "The editor's placeholder was not given what a reloaded node script declared as it ran again:\n#{output}"
      end

      # @behavior RS-057
      def verify_left_out!(output)
        return unless output.include?(UNTOUCHED)

        raise "The editor ran a file under a test directory:\n#{output}"
      end

      # @behavior RS-059
      def verify_tool_called!(output)
        called = output.lines.map(&:chomp).select { |line| TOOL_CALLBACKS.include?(line) }
        return if called == TOOL_CALLBACKS

        raise "The editor did not give a tool script's node its callbacks in order:\n#{output}"
      end

      # @behavior RS-060
      def verify_kept!(output)
        return if output.lines.map(&:chomp).include?(KEPT)

        raise "The editor lost a scene's value for a tool script that does not parse:\n#{output}"
      end

      def open_scene(project, scene)
        output, status = keep_layout(project) do
          Open3.capture2e(EXECUTABLE, "--headless", "--editor", "--verbose",
                          "--path", project, scene, "--quit-after", FRAMES)
        end
        raise "The editor did not run:\n#{output}" unless status.success?
        raise "The editor did not reach #{scene}:\n#{output}" unless output.include?(format(REACHED, scene))

        output
      end

      def keep_layout(project)
        layout = File.join(project, LAYOUT)
        kept = File.exist?(layout) ? File.binread(layout) : nil
        yield
      ensure
        kept ? File.binwrite(layout, kept) : FileUtils.rm_f(layout)
      end
    end
  end
end
