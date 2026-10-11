module Minitest
  # Frames a test drives itself rather than waits for, as GUT's simulate
  # does: a node's processing is called without the engine's frames.
  module Simulation
    def simulate(node, times, delta, check_is_processing = false)
      times.times do
        if node.has_method(:_process) && (!check_is_processing || node.is_processing)
          node.call(:_process, delta)
        end
        if node.has_method(:_physics_process) && (!check_is_processing || node.is_physics_processing)
          node.call(:_physics_process, delta)
        end
        node.get_children.each { |child| simulate(child, 1, delta, check_is_processing) }
      end
    end
  end

  class Test
    include Simulation
  end
end
