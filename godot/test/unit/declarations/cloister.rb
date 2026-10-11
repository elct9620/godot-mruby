module Unit
  module Declarations
    # A node class that defines a method as its body runs and makes it
    # private, so only its run says the class has it.
    class Cloister < Godot::Node
      define_method(:vespers) { "sung" }
      private :vespers
    end
  end
end
