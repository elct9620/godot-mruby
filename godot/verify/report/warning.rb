# The compiler warns at the block's end that this else is useless; it runs.
module Verify
  module Report
    class Warning < Godot::Node
      begin
        :body
      else
        :useless
      end
    end
  end
end
