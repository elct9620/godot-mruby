# frozen_string_literal: true

module Godot
  # Runs the integration-test project's scene that asks a node script what
  # Godot asks before its file runs, and reads what it prints: the kind of
  # script it is, the methods it has, the signal a scene connects to and the
  # properties Godot reads and lists — every one of them answered from a
  # script's source while no file runs. Part of NodeScripts.verify!.
  module Header
    # The scene, the answers it prints, and the lines a file prints if it
    # ever runs.
    SCENE = "res://verify/script/header/header.tscn"
    BASE_TYPE_ANSWER = "instance base type: Node2D"
    METHOD_ANSWER = "has _ready: true"
    TOOL_ANSWER = "is tool: true"
    ABSTRACT_ANSWER = "is abstract: true"
    SIGNAL_ANSWER = "has wailed: true"
    CONNECTED_ANSWER = "wailed connected: true"
    EXPORT_ANSWER = "volume: 11"
    EXPORT_LISTED_ANSWER = "volume listed: true"
    # A value the class builds as it runs is not one the source writes, so
    # the script has none to answer with until the file has run.
    UNWRITTEN_EXPORT_ANSWER = "heard at: <null>"
    INHERITED_BASE_TYPE_ANSWER = "inherited base type: Node2D"
    BASE_SCRIPT_ANSWER = "base script: res://verify/script/inherit/enemy.rb"
    RAN_LINES = ["reported.rb ran", "marked.rb ran", "siren.rb ran"].freeze

    module_function

    # Runs the scene that asks a node script what Godot asks and reads its
    # answers, each of which has to come from a script's source while the
    # files it asks about never run.
    def verify!(project)
      output, status = Godot.run_scene(project, SCENE, "--quit-after", "3")
      lines = output.lines.map(&:chomp)
      raise "The header scene did not run:\n#{output}" unless status.success?
      raise "The header scene ran a file it only asked about:\n#{output}" if lines.intersect?(RAN_LINES)

      verify_script_answers!(lines, output)
      verify_declaration_answers!(lines, output)
    end

    # What the script itself is: the engine class it extends, the methods it
    # has, and the marks its class body writes.
    def verify_script_answers!(lines, output)
      verify_base_type_answered!(lines, output)
      verify_method_answered!(lines, output)
      verify_tool_answered!(lines, output)
      verify_abstract_answered!(lines, output)
      verify_inherited_base_type_answered!(lines, output)
      verify_base_script_answered!(lines, output)
    end

    # What its class declares: the signal a scene connects to, and the
    # properties Godot reads and lists.
    def verify_declaration_answers!(lines, output)
      verify_signal_answered!(lines, output)
      verify_signal_connected!(lines, output)
      verify_export_answered!(lines, output)
      verify_export_listed!(lines, output)
      verify_unwritten_export_answered!(lines, output)
    end

    # @behavior RS-008
    def verify_base_type_answered!(lines, output)
      verify_answer!(lines, output, BASE_TYPE_ANSWER)
    end

    # @behavior RS-009
    def verify_method_answered!(lines, output)
      verify_answer!(lines, output, METHOD_ANSWER)
    end

    # @behavior RS-018
    def verify_tool_answered!(lines, output)
      verify_answer!(lines, output, TOOL_ANSWER)
    end

    # @behavior RS-019
    def verify_abstract_answered!(lines, output)
      verify_answer!(lines, output, ABSTRACT_ANSWER)
    end

    # @behavior RS-021
    def verify_inherited_base_type_answered!(lines, output)
      verify_answer!(lines, output, INHERITED_BASE_TYPE_ANSWER)
    end

    # @behavior RS-022
    def verify_base_script_answered!(lines, output)
      verify_answer!(lines, output, BASE_SCRIPT_ANSWER)
    end

    # @behavior RS-038
    def verify_signal_answered!(lines, output)
      verify_answer!(lines, output, SIGNAL_ANSWER)
    end

    # @behavior RS-039
    def verify_signal_connected!(lines, output)
      verify_answer!(lines, output, CONNECTED_ANSWER)
    end

    # @behavior RS-040
    def verify_export_answered!(lines, output)
      verify_answer!(lines, output, EXPORT_ANSWER)
    end

    # @behavior RS-041
    def verify_export_listed!(lines, output)
      verify_answer!(lines, output, EXPORT_LISTED_ANSWER)
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
