extends Node


# Calls the Answers node's method for what cannot cross, printing the type
# and value Godot got back.
func _ready() -> void:
	var answers := get_node("../Answers")
	var answer: Variant = answers.call("make_self_loop")
	print("make_self_loop answered ", type_string(typeof(answer)), " ", answer)
