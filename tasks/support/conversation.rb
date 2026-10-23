# frozen_string_literal: true

require "open3"

module Godot
  # Runs a command, typing to it what it is answered as it prints, and kills
  # a run that hangs. Backs Godot.capture.
  module Conversation
    # A run that outlasts this, or prints past the limit, is taken as hung
    # and killed, so its check fails with its output instead of waiting for
    # the CI job's limit; no check's run comes near either.
    TIMEOUT = 300
    OUTPUT_LIMIT = 64 * 1024 * 1024
    PIECE = 64 * 1024

    module_function

    # What `command` printed, and how it ended or nil when it was killed as
    # hung. `stdin_data` is typed first; without `respond` the input closes
    # then, as Open3.capture2e closes it.
    def hold(command, stdin_data, respond, **)
      Open3.popen2e(*command, **) do |input, output, wait|
        type(input, stdin_data)
        input.close unless respond
        printed, ended = read_output(output) { |text| type(input, respond.call(text)) if respond }
        kill(wait.pid) unless ended
        [printed.force_encoding(Encoding.default_external), (wait.value if ended)]
      end
    end

    # Reads `output` until the run closes it, yielding all read so far at
    # each piece, and answers it with whether the run ended within the limits.
    def read_output(output)
      pieces = Thread::Queue.new
      Thread.new { pump(output, pieces) }
      printed = String.new(encoding: Encoding::BINARY)
      deadline = now + TIMEOUT
      while (piece = pieces.pop(timeout: [deadline - now, 0].max))
        printed << piece
        return [printed, false] if printed.bytesize > OUTPUT_LIMIT

        yield printed
      end
      [printed, pieces.closed?]
    end

    def pump(output, pieces)
      loop { pieces << output.readpartial(PIECE) }
    rescue IOError
      pieces.close
    end

    def type(input, text)
      input.write(text) if text
    rescue Errno::EPIPE, Errno::EINVAL
      nil
    end

    def kill(pid)
      Process.kill(:KILL, pid)
    rescue Errno::ESRCH
      nil
    end

    def now
      Process.clock_gettime(Process::CLOCK_MONOTONIC)
    end
  end
end
