module Unit
  module Reload
    # A node script inheriting a class whose source a test changes.
    class Moth < Larva
      def lineage
        "#{self.class.superclass} < #{self.class.superclass.superclass}"
      end
    end
  end
end
