module Unit
  module EngineClasses
    # An abstract node class handing `abstract` names whose class methods
    # Ruby lists apart from their objects': ones `private` hides, a `def
    # self.` the singleton class's own singleton class takes, one
    # `attr_reader` gives the objects too, and ones the class's name, an
    # alias or `public` leaves on the class.
    class Seal < Godot::Node
      abstract

      def self.p
        "class p"
      end

      class << self
        def v
          "class v"
        end
        private :v, :p

        def self.r
          "meta r"
        end

        Seal.define_singleton_method(:q) { "class q" }
        EngineClasses::Seal.define_singleton_method(:k) { "class k" }

        def w
          "class w"
        end
        alias u w

        def m
          "class m"
        end
        private :m
        public :m
      end

      attr_reader :t

      def self.t
        "class t"
      end

      abstract :v, :r, :t, :q, :k, :p, :u, :m
    end
  end
end
