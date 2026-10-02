# Prints where user:// is for this project, so a check outside Godot can
# write the run file the editor's test panel leaves and see it removed.
module Integration
  module Runner
    module RunFile
      class UserDir < Godot::Node
        def _ready
          puts "user_dir=#{Godot::OS.get_user_data_dir}"
          get_tree.quit
        end
      end
    end
  end
end
