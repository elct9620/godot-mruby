module Unit
  module Loader
    # Needs Unit::Loader::CycleB before it defines itself, and cycle_b.rb needs
    # it back: a cycle the realm names rather than recursing.
    CycleA = CycleB
  end
end
