# A node script only through enemy.rb, and with no method of its own.
module Verify
  module Script
    module Inherit
      class Boss < Enemy
      end
    end
  end
end
