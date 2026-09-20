module Loader
  # A node class that rings: its signal carries how many times it rang.
  class Bell < Godot::Node
    signal :rung, :times

    def ring(times)
      emit_signal(:rung, times)
    end
  end
end
