module Unit
  module Declarations
    # Lets peers call one method at `@rpc`'s defaults and another with each
    # word set, counting the calls that ran here.
    class Courier < Godot::Node
      rpc :deliver
      rpc :announce, mode: :any_peer, sync: :call_local, transfer_mode: :unreliable_ordered, transfer_channel: 2

      attr_reader :announced

      def deliver; end

      def announce
        @announced = (@announced || 0) + 1
      end
    end
  end
end
