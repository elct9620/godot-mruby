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
      waited += next_physics_delta while waited < time
    end

    def wait_until(max_time, time_between = 0.0)
      raise ArgumentError, "wait_until needs a block to wait on" unless block_given?

      waited = since_called = 0.0
      loop do
        delta = next_physics_delta
        return false if (waited += delta) >= max_time
        next if (since_called += delta) < time_between

        since_called = 0.0
        return true if yield == true
      end
    end

    def wait_for_signal(signal, max_time)
      emitted = false
      connection = connect_block(signal) { emitted = true }
      wait_until(max_time) { emitted }
    ensure
      # A signal whose object was freed meanwhile went with its connections.
      signal.disconnect(connection) if connection && signal.get_object
    end

    private

    # Connects the block to `signal`, answering the engine's Callable for the
    # connection: a Proc becomes a new Callable each time it reaches the
    # engine, so only the one the engine holds disconnects it.
    def connect_block(signal, &block)
      held = connected_callables(signal)
      signal.connect(block)
      connected_callables(signal).find { |callable| !held.include?(callable) }
    end

    def connected_callables(signal)
      signal.get_connections.map { |connection| connection["callable"] }
    end

    # A test that waited into a physics frame leaves the next one there, so
    # each test begins where a process frame begins, as the first one does.
    def begin_at_process_frame
      next_frame(:process) unless Minitest.frame == :process
    end

    # Hands frames back until a physics frame begins, answering the delta it
    # gives _physics_process: the engine's time scale over its physics ticks
    # per second.
    def next_physics_delta
      next_frame(:physics)
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
