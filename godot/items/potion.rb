module Items
  # An item priced from what its namespace's file defines, reaching the label
  # beside it and the inventory at the top by name.
  class Potion
    def price
      BASE_PRICE
    end

    def label
      Label.new(self).text
    end

    def stocked(slots)
      Inventory.new(slots).add(self)
    end
  end
end
