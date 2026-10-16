module Unit
  module Declarations
    # Exports a property naming a resource class the project announces.
    class Almanac < Godot::Node
      export :page, nil, type: Integration::Declarations::Exports::LedgerPage
    end
  end
end
