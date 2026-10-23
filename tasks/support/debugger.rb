# frozen_string_literal: true

module Godot
  # Runs the debugger scene under Godot's command-line debugger, which takes
  # the commands typed at each break from standard input, and reads where it
  # breaks. Part of Godot.verify!.
  #
  # Each command is typed once the debugger asks for it: on Windows a read
  # takes all the input waiting, so commands typed at once read as one.
  module Debugger
    SCENE = "res://integration/debugger/debugger.tscn"
    SCRIPT = "res://integration/debugger/stepper.rb"
    TALLY = "res://integration/debugger/tally.rb"
    # A break that nothing expected quits the game rather than waiting on
    # input that never comes.
    QUIT = "q"
    PROMPT = "debug> "
    # The breakpoint stepping starts from, the commands typed at its breaks,
    # and the places the stepper breaks at in turn: its next line, the method
    # its next line calls in another file, then back out at the line after
    # the call.
    STEPS = { breakpoint: "#{SCRIPT}:8", commands: %w[s s bt o c],
              places: ["#{SCRIPT}:8", "#{SCRIPT}:9", "#{TALLY}:10", "#{SCRIPT}:10"] }.freeze
    # The stack at the break in the called method.
    STACK = ["*Frame 0 - #{TALLY}:10 in function ''", " Frame 1 - #{SCRIPT}:9 in function ''"].freeze
    # A step over the call breaks at the line after it.
    STEP_OVER = { breakpoint: "#{SCRIPT}:9", commands: %w[n c], places: ["#{SCRIPT}:9", "#{SCRIPT}:10"] }.freeze
    # A step into the call that loads a file by name breaks in that file, not
    # in the extension's loader, and steps out of it back to the callback.
    LOADING = { breakpoint: "#{SCRIPT}:7", commands: %w[s o c],
                places: ["#{SCRIPT}:7", "#{TALLY}:3", "#{SCRIPT}:8"] }.freeze
    # A breakpoint in another file than the one the callback starts in.
    ELSEWHERE = { breakpoint: "#{TALLY}:11", commands: %w[c], places: ["#{TALLY}:11"] }.freeze
    END_LINE = "stepped to 3"

    module_function

    def verify!(project)
      verify_steps!(project)
      verify_step_over!(project)
      verify_elsewhere!(project)
      verify_loading!(project)
    end

    # @behavior SA-001 SA-002 SA-003 SA-005 SA-007
    def verify_steps!(project)
      lines = debug(project, STEPS)
      return if breaks(lines) == STEPS[:places] && stacked?(lines) && lines.last(1) == [END_LINE]

      raise "The debugger did not break at #{STEPS[:places]} and run on:\n#{lines.join("\n")}"
    end

    # @behavior SA-004
    def stacked?(lines)
      lines.each_cons(STACK.size + 1).any? { |*frames, after| frames == STACK && !after.include?("Frame") }
    end

    # @behavior SA-006
    def verify_step_over!(project)
      lines = debug(project, STEP_OVER)
      return if breaks(lines) == STEP_OVER[:places]

      raise "The debugger did not step over the call to #{STEP_OVER[:places]}:\n#{lines.join("\n")}"
    end

    # @behavior SA-001
    def verify_elsewhere!(project)
      lines = debug(project, ELSEWHERE)
      return if breaks(lines) == ELSEWHERE[:places]

      raise "The debugger did not break at #{ELSEWHERE[:places]}:\n#{lines.join("\n")}"
    end

    # @behavior SA-010
    def verify_loading!(project)
      lines = debug(project, LOADING)
      return if breaks(lines) == LOADING[:places]

      raise "The debugger did not step through loading to #{LOADING[:places]}:\n#{lines.join("\n")}"
    end

    # The release template's library has no debug hook, so a breakpoint in
    # an exported game never stops it.
    # @behavior SA-008
    def verify_release!(dir)
      output, = Release.play(dir, SCENE, *under_debugger(STEPS), stdin_data: QUIT)
      return if !output.include?("Debugger Break") && output.lines.map(&:chomp).include?(END_LINE)

      raise "The release template's game stopped at a breakpoint or did not run on:\n#{output}"
    end

    # The debug template's library stops an exported game as the editor's
    # stops one it plays.
    # @behavior SA-009
    def verify_debug!(dir)
      output, = Release.play(dir, SCENE, *under_debugger(STEPS), respond: answer(STEPS))
      return if breaks(read_breaks(output)) == STEPS[:places]

      raise "The debug template's game did not break at #{STEPS[:places]}:\n#{output}"
    end

    # What the scene printed under the debugger, from its first break.
    def debug(project, run)
      output, = Godot.run_scene(project, SCENE, "--quit-after", "3", *under_debugger(run), respond: answer(run))
      read_breaks(output)
    end

    # The arguments that play a scene under the command-line debugger with
    # the breakpoint of `run`.
    def under_debugger(run)
      ["-d", "-b", run[:breakpoint]]
    end

    # Answers each prompt the debugger has printed with the next command of
    # `run`, then with QUIT.
    def answer(run)
      commands = run[:commands].dup
      answered = 0
      lambda do |output|
        next if output.scan(PROMPT).size == answered

        answered += 1
        "#{commands.shift || QUIT}\n"
      end
    end

    # The lines `output` holds from its first break, each without the prompt
    # the debugger writes before reading a command.
    def read_breaks(output)
      lines = output.lines.map { |line| line.chomp.delete_prefix(PROMPT) }.reject(&:empty?)
      lines.drop_while { |line| !line.start_with?("Debugger Break") }
    end

    # The file and line of each break.
    def breaks(lines)
      lines.each_cons(2).filter_map do |line, frame|
        frame[/\A\*Frame 0 - (\S+:\d+) /, 1] if line.start_with?("Debugger Break")
      end
    end
  end
end
