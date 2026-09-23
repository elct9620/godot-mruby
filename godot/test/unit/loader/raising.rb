module Unit
  module Loader
    # Defines its own constant and one beside it, loads kept.rb by name, reaches
    # Unit::Loader::Shelf::Box, whose directory has no file of its own, then
    # raises: what it created leaves with it, and the rest stays.
    class Raising
    end

    class RaisingError < StandardError; end

    Kept
    Shelf::Box
    raise "raising.rb fails after defining its constants"
  end
end
