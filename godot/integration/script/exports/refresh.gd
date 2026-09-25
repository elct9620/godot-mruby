@tool
extends Node

const CHANGED := """module Integration
  module Script
    module Exports
      class Plated < Godot::Node2D
        export :plates, 1
        export :armor, 7
      end
    end
  end
end
"""


# In the editor, changes the Ruby node script's source and reloads it, then
# prints whether the node's placeholder lists the property it now exports;
# its value would come from the script whether or not the placeholder was told.
func _ready() -> void:
	# A person's editor is left alone: the probe changes a script's source,
	# which a save there would write to the file.
	if not Engine.is_editor_hint() or DisplayServer.get_name() != "headless":
		return
	var script: Script = $Plated.get_script()
	script.source_code = CHANGED
	script.reload()
	var names := $Plated.get_property_list().map(func(property): return property.name)
	print("armor listed: ", names.has("armor"))
