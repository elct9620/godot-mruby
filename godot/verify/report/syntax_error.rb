# The compiler stops at the stray end below, so this file never runs.
module Verify
  module Report
    class SyntaxError < Godot::Node
      puts "syntax_error.rb ran"

      def _ready
      end
    end
  end
end
end
