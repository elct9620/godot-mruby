module Unit
  module Script
    # Awaits a signal for the class taking it in, from a module's own method.
    module Listener
      def listen(source)
        await(source, :relayed)
      end
    end
  end
end
