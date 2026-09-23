module Integration
  module Script
    module Callbacks
      class Notified < Godot::Node
        # 13 is NOTIFICATION_READY.
        def _notification(what)
          puts "notified.rb was notified it is ready" if what == 13
        end
      end
    end
  end
end
