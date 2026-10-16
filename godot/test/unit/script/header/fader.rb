module Unit
  module Script
    module Header
      # Exports a range with extra hints and a tool button, and lets peers
      # call a method, which Godot is told without the file running.
      class Fader < Godot::Node2D
        export :level, 5, range: 0..10, extra_hints: [:or_greater]
        export :reset, :restart, tool_button: "Reset"
        rpc :restart, mode: :any_peer

        def restart; end
      end
    end
  end
end
