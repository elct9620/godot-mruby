module Unit
  module Script
    # Made by GDScript as by Ruby, heated as it is made, and tempering through
    # a method of the class itself.
    class Forge < Godot::RefCounted
      attr_reader :heat

      def initialize(heat = 1)
        @heat = heat
      end

      def self.temper(value)
        value * 2
      end
    end
  end
end
