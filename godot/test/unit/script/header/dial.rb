module Unit
  module Script
    module Header
      # Shows its properties under a group and with a hint, which the editor
      # is told without the file running.
      class Dial < Godot::Node2D
        export_group "Aim"
        export :angle, 0.0, range: 0.0..360.0
      end
    end
  end
end
