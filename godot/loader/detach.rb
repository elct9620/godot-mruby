module Loader
  # A node class whose method tries to take its node's script away.
  class Detach < Godot::Node
    def detach
      set_script(nil)
      :detached
    rescue Godot::CallError => e
      e.message
    end
  end
end
