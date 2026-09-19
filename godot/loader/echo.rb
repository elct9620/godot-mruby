module Loader
  # A node class whose method calls back into its own node.
  class Echo < Godot::Node
    attr_reader :notified

    def _notification(what)
      @notified = what
    end

    def ping
      notification(7)
      :pinged
    end
  end
end
