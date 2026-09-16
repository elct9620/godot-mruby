module Verify
  module Script
    module Callbacks
      class Process < Godot::Node
        def _process(delta)
          return if @printed

          @printed = true
          puts "process.rb was given a #{delta.class}"
        end
      end
    end
  end
end
