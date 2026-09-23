# frozen_string_literal: true

module Godot
  # Runs the integration-test project's scene that reads a node script's
  # exported properties before its file runs, and reads what it prints. The
  # scene reads them from GDScript, since a read from inside the realm builds
  # the node's object, which runs its file. Part of NodeScripts.verify!.
  module Header
    # The scene, the answers it prints, and the line the file prints if it
    # ever runs.
    SCENE = "res://integration/script/header/header.tscn"
    EXPORT_ANSWER = "volume: 11"
    # A value the class builds as it runs is not one the source writes, so
    # the script has none to answer with until the file has run.
    UNWRITTEN_EXPORT_ANSWER = "heard at: <null>"
    RAN_LINE = "siren.rb ran"

    module_function

    # Runs the scene and reads its answers, each of which has to come from
    # the script's source while its file never runs.
    def verify!(project)
      output, status = Godot.run_scene(project, SCENE, "--quit-after", "3")
      lines = output.lines.map(&:chomp)
      raise "The header scene did not run:\n#{output}" unless status.success?
      raise "The header scene ran a file it only asked about:\n#{output}" if lines.include?(RAN_LINE)

      verify_export_answered!(lines, output)
      verify_unwritten_export_answered!(lines, output)
    end

    # @behavior RS-040
    def verify_export_answered!(lines, output)
      verify_answer!(lines, output, EXPORT_ANSWER)
    end

    # @behavior RS-042
    def verify_unwritten_export_answered!(lines, output)
      verify_answer!(lines, output, UNWRITTEN_EXPORT_ANSWER)
    end

    def verify_answer!(lines, output, answer)
      return if lines.include?(answer)

      raise "The header scene did not answer #{answer.inspect}:\n#{output}"
    end
  end
end
