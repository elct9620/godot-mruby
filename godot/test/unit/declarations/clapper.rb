module Unit
  module Declarations
    # What strikes a bell, as a module a node class takes in: a method and an
    # engine callback that are none of the class's own.
    module Clapper
      def strike = "struck"

      def _notification(what)
        @last_notification = what
      end
    end
  end
end
