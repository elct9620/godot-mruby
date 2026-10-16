module Unit
  module Script
    module Header
      # Exports a range with extra hints and a tool button, which the editor
      # is told without the file running.
      class Fader < Godot::Node2D
        export :level, 5, range: 0..10, extra_hints: [:or_greater]
        export :reset, :restart, tool_button: "Reset"

        def restart; end
      end
    end
  end
end
