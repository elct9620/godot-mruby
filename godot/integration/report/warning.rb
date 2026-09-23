# The compiler warns at the block's end that this else is useless; it runs.
module Integration
  module Report
    class Warning < Godot::Node
      begin
        :body
      else
        :useless
      end

      def _ready
      end
    end
  end
end
