module Unit
  module Script
    module Header
      # A node script whose file a test asks about and never runs.
      class Dossier < Godot::Node2D
        def _ready
        end
      end
    end
  end
end
