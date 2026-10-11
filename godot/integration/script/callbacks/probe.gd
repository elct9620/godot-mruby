extends Node


# Calls the Answers node's methods with and for what cannot cross, printing
# the type and value Godot got back.
func _ready() -> void:
	var answers := get_node("../Answers")
	var answer: Variant = answers.call("looped")
	print("looped answered ", type_string(typeof(answer)), " ", answer)
	var looped := []
	looped.append(looped)
	answers.call("take", looped)
