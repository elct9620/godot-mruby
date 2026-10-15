module Unit
  module Declarations
    # Declares a signal the class it extends declared.
    class Resonator < Bell
      signal :rung, times: Integer
    end
  end
end
