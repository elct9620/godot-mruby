module Unit
  module EngineClasses
    # An abstract node class handing `abstract` names as Ruby evaluates the
    # call: a `def self.` beside an instance method of its name, a `def
    # self.` among the call's own arguments, and one `class << self` defines.
    class Emblem < Godot::Node
      abstract

      def x
        "x"
      end
      abstract def self.x
        "class x"
      end

      abstract :y, def self.y
        "class y"
      end

      class << self
        def z
          "class z"
        end
      end
      abstract :z
    end
  end
end
