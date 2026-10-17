# Adds a node script named as the announced boss.rb while the editor runs,
# then scans the project, printing whether the editor lists a class by that
# name before the file came and after the scan took it in.
module Integration
  module Announcement
    module Rescan
      class RescanProbe < Godot::EditorPlugin
        tool

        NAME = :Boss
        NEWCOMER = "res://integration/announcement/rescan/newcomer/boss.rb".freeze
        SOURCE = <<~RUBY.freeze
          module Integration
            module Announcement
              module Rescan
                module Newcomer
                  class Boss < Godot::Node
                  end
                end
              end
            end
          end
        RUBY
        ADDITION_FRAME = 60
        REPORT_FRAME = 150

        def _ready
          @frames = 0
        end

        def _process(_delta)
          @frames += 1
          case @frames
          when ADDITION_FRAME then add
          when REPORT_FRAME then report("after the scan")
          end
        end

        def add
          report("before the file came")
          Godot::DirAccess.make_dir_recursive_absolute(NEWCOMER[0, NEWCOMER.rindex("/")])
          file = Godot::FileAccess.open(NEWCOMER, Godot::FileAccess::WRITE)
          file.store_string(SOURCE)
          file.close
          Godot::EditorInterface.get_resource_filesystem.scan
        end

        def report(moment)
          listed = Godot::ProjectSettings.get_global_class_list.any? { |entry| entry["class"] == NAME }
          puts "#{NAME} listed #{moment}: #{listed}"
        end
      end
    end
  end
end
