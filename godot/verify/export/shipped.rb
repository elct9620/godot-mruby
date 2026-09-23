# Prints each path after `--` that the running game has a resource at.
module Verify
  module Export
    class Shipped < Godot::Node
      def _ready
        Godot::OS.get_cmdline_user_args.each do |path|
          puts "shipped: #{path}" if Godot::ResourceLoader.exists(path)
        end
      end
    end
  end
end
