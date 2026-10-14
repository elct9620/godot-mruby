module Unit
  module Loader
    class Chest
      # Raises as chest.rb creates its own class.
      class Hinge
      end

      raise "the hinge does not fit"
    end
  end
end
