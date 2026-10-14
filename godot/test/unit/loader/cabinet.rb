module Unit
  module Loader
    # Opens its own Drawer, whose file cabinet/drawer.rb has to have run
    # first.
    class Cabinet
      class Drawer
        def shut?
          true
        end
      end
    end
  end
end
