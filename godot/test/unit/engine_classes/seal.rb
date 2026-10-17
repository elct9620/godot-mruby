module Unit
  module EngineClasses
    # An abstract node class handing `abstract` names whose class methods
    # Ruby lists apart from their objects': one `private` hides, one `def
    # self.` puts on the singleton class's own, one `attr_reader` gives the
    # objects too, and one the class's name defines on the class.
    class Seal < Godot::Node
      abstract

      class << self
        def v
          "class v"
        end
        private :v

        def self.r
          "meta r"
        end

        Seal.define_singleton_method(:q) { "class q" }
      end

      attr_reader :t

      def self.t
        "class t"
      end

      abstract :v, :r, :t, :q
    end
  end
end
