module Unit
  module Declarations
    # Exports a property with each hint the rest of GDScript's export
    # annotations give, and typed containers.
    class Console < Godot::Node
      export :tint, Godot::Color.new(1, 0, 0), color_no_alpha: true
      export :fade, 1.0, exp_easing: %w[attenuation positive_only]
      export :icon_path, "", file_path: "*.png"
      export :log_file, "", global_file: "*.log"
      export :save_dir, "", global_dir: true
      export :hit_layers, 1, flags_2d_physics: true
      export :avoid_layers, 1, flags_avoidance: true
      export :target, Godot::NodePath.new(""), node_path: [Godot::Button, Godot::Label]
      export :secret, 0, storage: true
      export :reset, :restart, tool_button: "Reset"
      export :action, "", custom: [43, "show_builtin", 6 | 268_435_456]
      export :volume, 50, range: 0..100, extra_hints: [:or_greater, "suffix:dB"]
      export :scores, [], type: [Integer]
      export :sprites, [], type: [Godot::Texture2D]
      export :prices, {}, type: { String => Integer }

      def restart
        "restarted"
      end
    end
  end
end
