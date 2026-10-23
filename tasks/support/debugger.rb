# frozen_string_literal: true

module Godot
  # Runs the debugger scene under Godot's command-line debugger, which takes
  # the commands typed at each break from standard input, and reads where it
  # breaks. Part of Godot.verify!.
  module Debugger
    SCENE = "res://integration/debugger/debugger.tscn"
    SCRIPT = "res://integration/debugger/stepper.rb"
    # A break that nothing expected quits the game rather than waiting on
    # input that never comes.
    QUIT = "q"
    # The line stepping starts from, the commands typed at its breaks, and the
    # lines the stepper breaks at in turn: its next line, the method its next
    # line calls, then back out at the line after the call.
    STEPS = { breakpoint: 7, commands: %w[s s bt o c], lines: [7, 8, 12, 9] }.freeze
    # The stack at the break in the called method.
    STACK = ["*Frame 0 - #{SCRIPT}:12 in function ''", " Frame 1 - #{SCRIPT}:8 in function ''"].freeze
    # A step over the call breaks at the line after it.
    STEP_OVER = { breakpoint: 8, commands: %w[n c], lines: [8, 9] }.freeze
    END_LINE = "stepped to 2"

    module_function

    def verify!(project)
      verify_steps!(project)
      verify_step_over!(project)
    end

    # @behavior SA-001 SA-002 SA-003 SA-005 SA-007
    def verify_steps!(project)
      lines = debug(project, STEPS)
      return if breaks(lines) == STEPS[:lines] && stacked?(lines) && lines.last(1) == [END_LINE]

      raise "The debugger did not break at #{STEPS[:lines]} and run on:\n#{lines.join("\n")}"
    end

    # @behavior SA-004
    def stacked?(lines)
      lines.each_cons(STACK.size + 1).any? { |*frames, after| frames == STACK && !after.include?("Frame") }
    end

    # @behavior SA-006
    def verify_step_over!(project)
      lines = debug(project, STEP_OVER)
      return if breaks(lines) == STEP_OVER[:lines]

      raise "The debugger did not step over the call to #{STEP_OVER[:lines]}:\n#{lines.join("\n")}"
    end

    # What the scene printed under the debugger, from its first break, each
    # line without the prompt the debugger writes before reading a command.
    def debug(project, run)
      output, = Godot.run_scene(project, SCENE, "--quit-after", "3", "-d", "-b", "#{SCRIPT}:#{run[:breakpoint]}",
                                stdin_data: [*run[:commands], QUIT].join("\n"))
      lines = output.lines.map { |line| line.chomp.delete_prefix("debug> ") }.reject(&:empty?)
      lines.drop_while { |line| !line.start_with?("Debugger Break") }
    end

    # The stepper's line at each break.
    def breaks(lines)
      lines.each_cons(2).filter_map do |line, frame|
        frame[/\A\*Frame 0 - #{Regexp.escape(SCRIPT)}:(\d+) /, 1]&.to_i if line.start_with?("Debugger Break")
      end
    end
  end
end
