module Unit
  module Declarations
    # Exports a name the class it extends exported.
    class Decoy < Turret
      export :mode, :first
    end
  end
end
