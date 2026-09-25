@tool
extends Node


# In the editor, waits for the extension to run the knob's file at a frame,
# then prints the hint and the group its placeholder lists; then widens the
# knob's bounds, reloads it, and prints the hint once the file ran again.
func _ready() -> void:
	if not Engine.is_editor_hint():
		return
	await frames()
	var grouped := listed().any(
		func(property): return property.name == "Turning" and property.usage == PROPERTY_USAGE_GROUP
	)
	print("knob turn hint: ", turn_hint(), ", grouped: ", grouped)
	var script: Script = $Knob.get_script()
	script.source_code = script.source_code.replace("0..10", "0..20")
	script.reload()
	await frames()
	print("knob turn hint after reload: ", turn_hint())


func frames() -> void:
	for frame in 3:
		await get_tree().process_frame


func listed() -> Array:
	return $Knob.get_property_list()


func turn_hint() -> String:
	return listed().filter(func(property): return property.name == "turn")[0].hint_string
