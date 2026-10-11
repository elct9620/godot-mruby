module Unit
  module Loader
    class Awning
    end

    # Opens the module of shades/, which has no file of its own, then raises.
    module Shades
    end

    raise "awning.rb fails after opening Unit::Loader::Shades"
  end
end
