module Unit
  module Script
    # Waits for its source's signals in the methods Godot calls on it,
    # noting each step and what each wait answered.
    class Sentinel < Godot::Node
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
    end
  end
end
