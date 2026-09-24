# A run spread over frames: the test runner resumes it as each process frame
# begins, and a test waits by handing the frame back until then.
module Minitest
  def self.start(options)
    @running = Fiber.new { run(options) }
    resume
  end

  def self.resume
    answer = @running.resume
    answer unless @running.alive?
  end

  # What a test waits for, each a call in the test's own code: a wait hands
  # the frame back through Ruby frames only, so not from a block a C method
  # calls.
  module Waits
    def wait_process_frames(frames)
      frames.times { Fiber.yield }
    end
  end

  class Test
    include Waits
  end
end
