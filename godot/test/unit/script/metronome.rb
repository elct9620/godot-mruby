module Unit
  module Script
    # A node class keeping the first delta its _process is given.
    class Metronome < Godot::Node
      attr_reader :delta

      def _process(delta)
        @delta ||= delta
      end
    end
  end
end
