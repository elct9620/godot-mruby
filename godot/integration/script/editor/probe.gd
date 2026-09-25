@tool
extends Node

# In the editor, waits for the extension to run the knob's file at a frame,
# then prints the hint and the group its placeholder lists.
func _ready() -> void:
	if not Engine.is_editor_hint():
		return
	for frame in 3:
		await get_tree().process_frame
	var listed: Array = $Knob.get_property_list()
	var turn: Dictionary = listed.filter(func(property): return property.name == "turn")[0]
	var grouped := listed.any(func(property): return property.name == "Turning" and property.usage == PROPERTY_USAGE_GROUP)
	print("knob turn hint: ", turn.hint_string, ", grouped: ", grouped)
