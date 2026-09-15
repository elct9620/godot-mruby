module Loader
  module Items
    # A price tag for an item.
    class Label
      def initialize(item)
        @item = item
      end

      def text
        "#{@item.class}: #{@item.price}"
      end
    end
  end
end
