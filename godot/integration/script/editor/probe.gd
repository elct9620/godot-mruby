@tool
extends Node


# In the editor, waits for the extension to run the knob's file at a frame,
# then prints the hint and the group its placeholder lists, and what a tool
# that does not parse kept of the scene; then widens the knob's bounds,
# reloads it, and prints the hint once the file ran again.
func _ready() -> void:
	# A person's editor is left alone: the probe changes a script's source,
	# which a save there would write to the file.
	if not Engine.is_editor_hint() or DisplayServer.get_name() != "headless":
		return
	await wait_frames()
	var grouped := properties().any(
		func(property): return property.name == "Turning" and property.usage == PROPERTY_USAGE_GROUP
	)
	print("knob turn hint: ", turn_hint(), ", grouped: ", grouped)
	print("cracked kept: ", $Cracked.get("kept"))
	var script: Script = $Knob.get_script()
	script.source_code = script.source_code.replace("0..10", "0..20")
	script.reload()
	await wait_frames()
	print("knob turn hint after reload: ", turn_hint())


func wait_frames() -> void:
	for frame in 3:
		await get_tree().process_frame


func properties() -> Array:
	return $Knob.get_property_list()


func turn_hint() -> String:
	return properties().filter(func(property): return property.name == "turn")[0].hint_string
