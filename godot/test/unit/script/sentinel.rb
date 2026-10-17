module Unit
  module Script
    # Waits for its source's signals in the methods Godot calls on it,
    # noting each step and what each wait answered.
    class Sentinel < Godot::Node
      include Listening

      attr_reader :log

      def initialize
        @log = []
      end

      def watch(source)
        @log << :started
        @log << await(source, :relayed)
        :done
      end

      def arm(source)
        source.connect(:pinged, proc { @log << await(source, :relayed) })
      end

      # Writes no await of its own: the methods it calls wait.
      def relay(source)
        @log << wait_for_relay(source)
        @log << listen(source)
      end

      def wait_for_relay(source)
        await(source, :relayed)
      end

      def echo(source)
        [:echoed, await(source, :relayed)]
      end

      def maybe(source, waits)
        return :early unless waits

        await(source, :relayed)
        :late
      end

      def await_plain
        await(3)
      end

      def spawn(source)
        @spawned = Godot::Coroutine.run do
          @log << await(source, :relayed)
          :block_done
        end
        @log << :went_on
        @spawned
      end

      def join
        @log << await(@spawned)
      end

      def sort_awaiting(source)
        begin
          [2, 1].sort { await(source, :relayed) || 0 }
        rescue FiberError
          @log << :refused
        end
        @log << await(source, :relayed)
      end
    end
  end
end
