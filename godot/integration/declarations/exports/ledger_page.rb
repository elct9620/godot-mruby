# A resource class the project announces, which an export may name. It sits
# outside the test directories, whose files are never announced.
module Integration
  module Declarations
    module Exports
      class LedgerPage < Godot::Resource
      end
    end
  end
end
