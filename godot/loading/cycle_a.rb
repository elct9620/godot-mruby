module Loading
  # Needs Loading::CycleB before it defines itself, and cycle_b.rb needs it
  # back: a cycle the realm names rather than recursing.
  CycleA = CycleB
end
