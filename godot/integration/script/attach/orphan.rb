# Extends a name no file spells, so no node takes this file as its script.
module Integration
  module Script
    module Attach
      class Orphan < Missing
      end
    end
  end
end
