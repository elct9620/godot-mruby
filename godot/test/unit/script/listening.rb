module Unit
  module Script
    # Awaits for the class taking it in, from a module's own method.
    module Listening
      def listen(source)
        await(source, :relayed)
      end
    end
  end
end
