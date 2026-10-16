module Unit
  module Script
    # A reference-counted object counting what it was told.
    class Tally < Godot::RefCounted
      export :count, 0

      def bump
        @count += 1
      end
    end
  end
end
