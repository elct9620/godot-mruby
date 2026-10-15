module Unit
  module Declarations
    # A node class that rings: its signal carries how many times it rang, and
    # the methods it defines as its body runs stand beside the ones written as
    # `def`.
    class Bell < Godot::Node
      signal :rung, times: Integer

      attr_reader :last_notification

      def ring(times)
        emit_signal(:rung, times)
      end

      define_method(:toll) { |times| ring(times) }

      define_method(:_notification) { |what| @last_notification = what }
    end
  end
end
