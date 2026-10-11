module Unit
  module Declarations
    # A node class whose methods come from the module it takes in.
    class Carillon < Godot::Node
      include Clapper

      def last_notification = @last_notification
    end
  end
end
