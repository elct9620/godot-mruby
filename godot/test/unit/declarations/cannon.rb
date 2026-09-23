module Unit
  module Declarations
    # A turret exporting one more property, so what it has is its own and what
    # the class it extends exported.
    class Cannon < Turret
      export :barrel, "long"
    end
  end
end
