# Counts the frames Godot processes it on and the threads it is processed
# on, so a test can watch where a node in a sub-thread process group runs.
module Unit
  module Entry
    class Treadmill < Godot::Node
      attr_reader :steps, :threads

      def initialize
        super
        @steps = 0
        @threads = []
      end

      def _process(_delta)
        @steps += 1
        thread = Godot::OS.get_thread_caller_id
        @threads << thread unless @threads.include?(thread)
      end
    end
  end
end
