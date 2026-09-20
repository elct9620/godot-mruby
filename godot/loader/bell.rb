module Loader
  # A node class that rings: its signal carries how many times it rang, and
  # the methods it defines as its body runs stand beside the ones written as
  # `def`.
  class Bell < Godot::Node
    signal :rung, :times

    attr_reader :noticed

    def ring(times)
      emit_signal(:rung, times)
    end

    define_method(:toll) { |times| ring(times) }

    define_method(:_notification) { |what| @noticed = what }
  end
end
