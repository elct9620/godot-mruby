module Unit
  module Reload
    # A node script whose changed source exports a property its objects may
    # already hold.
    class Armored < Godot::Node
      def build; end

      def wear
        @armor = 9
      end
    end
  end
end
