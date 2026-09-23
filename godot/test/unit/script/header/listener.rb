module Unit
  module Script
    module Header
      # What the scene connects the siren's signal to, whose method Godot
      # finds in the header; nothing emits the signal, so the file never runs.
      class Listener < Godot::Node
        def _on_wailed(times)
        end
      end
    end
  end
end
