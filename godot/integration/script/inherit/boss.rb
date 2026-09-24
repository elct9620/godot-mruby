# A node script only through grunt.rb, with no method of its own, and an
# icon beside the file.
module Integration
  module Script
    module Inherit
      class Boss < Grunt
        icon "boss.svg"
      end
    end
  end
end
