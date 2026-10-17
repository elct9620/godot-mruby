module Unit
  module Loader
    # Defines its own constant and one beside it, loads keepsake.rb by name, reaches
    # Unit::Loader::Shelf::Box, whose directory has no file of its own, then
    # raises: what it created leaves with it, and the rest stays, its count of
    # runs on the namespace included.
    @tripwire_runs = (@tripwire_runs || 0) + 1

    class Tripwire
    end

    class TripwireError < StandardError; end

    Keepsake
    Shelf::Box
    raise "tripwire.rb fails after defining its constants"
  end
end
