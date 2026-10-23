# The count the stepper keeps, in a file of its own, so a step into it
# crosses files.
module Integration
  module Debugger
    module Tally
      def self.start
        1
      end

      def self.bump(count)
        count + 1
      end
    end
  end
end
