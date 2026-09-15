module Loader
  # Needs Loader::CycleA, whose file is still running.
  CycleB = CycleA
end
