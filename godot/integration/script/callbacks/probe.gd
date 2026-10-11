extends Node


# Calls the Answers node's methods with and for what cannot cross, printing
# the type and value Godot got back.
func _ready() -> void:
	var answers := get_node("../Answers")
	var answer: Variant = answers.call("self_loop")
	print("self_loop answered ", type_string(typeof(answer)), " ", answer)
	var self_loop := []
	self_loop.append(self_loop)
	answers.call("take", self_loop)
