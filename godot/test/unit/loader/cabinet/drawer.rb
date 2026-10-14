module Unit
  module Loader
    class Cabinet
      # Reopened by cabinet.rb inside its own class.
      class Drawer
        def labelled?
          true
        end
      end
    end
  end
end
