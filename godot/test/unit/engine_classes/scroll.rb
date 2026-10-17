module Unit
  module EngineClasses
    # An abstract node class naming in `abstract` only a method it defines on
    # itself, though the class it extends has an instance method of that name.
    class Scroll < Tablet
      abstract

      def self.count
        0
      end
      abstract :count
    end
  end
end
