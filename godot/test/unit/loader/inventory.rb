module Unit
  module Loader
    # A library the tests reach by name: it holds items up to its slots.
    class Inventory
      # What adding to a full inventory raises.
      class Full < StandardError; end

      def initialize(slots)
        @slots = slots
        @items = []
      end

      def add(item)
        raise Full, "no slot is free" if @items.size >= @slots

        @items << item
        self
      end

      def size
        @items.size
      end
    end
  end
end
