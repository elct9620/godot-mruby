module Unit
  module Loader
    # Needs Unit::Loader::CycleA, whose file is still running.
    CycleB = CycleA
  end
end
