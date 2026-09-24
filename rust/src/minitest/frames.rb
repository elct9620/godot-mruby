# A run spread over frames: the test runner resumes it as each process frame
# and each physics frame begins, and a test waits by handing the frame back
# until the one it waits for.
module Minitest
  def self.start(options)
    @running = Fiber.new { run(options) }
    resume(:process)
  end

  def self.resume(frame)
    @frame = frame
    answer = @running.resume(frame)
    answer unless @running.alive?
  end

  # The kind of frame the run was last resumed in.
  def self.frame
    @frame
  end

  # What a test waits for, each a call in the test's own code: a wait hands
  # the frame back through Ruby frames only, so not from a block a C method
  # calls.
  module Waits
    def before_setup
      super
      begin_at_process_frame
    end

    def wait_process_frames(frames)
      frames.times { next_frame(:process) }
    end

    def wait_physics_frames(frames)
      frames.times { next_frame(:physics) }
    end

    def wait_seconds(time)
      waited = 0.0
      while waited < time
        next_frame(:physics)
        waited += physics_delta
      end
    end

    private

    # A test that waited into a physics frame leaves the next one there, so
    # each test begins where a process frame begins, as the first one does.
    def begin_at_process_frame
      next_frame(:process) unless Minitest.frame == :process
    end

    # What each physics frame gives _physics_process: the engine's time scale
    # over its physics ticks per second.
    def physics_delta
      Godot::Engine.get_main_loop.root.get_physics_process_delta_time
    end

    # Hands frames back until one of the kind `frame` names begins.
    def next_frame(frame)
      nil until Fiber.yield == frame
    end
  end

  class Test
    include Waits
  end
end
