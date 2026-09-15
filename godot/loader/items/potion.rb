module Loader
  module Items
    # An item priced from what its namespace's file defines, reaching the label
    # beside it by name.
    class Potion
      def price
        BASE_PRICE
      end

      def label
        Label.new(self).text
      end
    end
  end
end
