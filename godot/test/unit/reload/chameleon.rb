module Unit
  module Reload
    # A node script whose changed source extends another engine class, given
    # to nodes of a class both reach.
    class Chameleon < Godot::Node
      export :hue, 1

      def paint
        @hue = 7
      end

      def kind
        "node"
      end
    end
  end
end
