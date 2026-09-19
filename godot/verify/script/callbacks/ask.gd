extends Node


# Calls the Answers node's methods and prints the type and value Godot got
# back from each.
func _ready() -> void:
	var answers := get_node("../Answers")
	for method in ["ratio", "count"]:
		var answer: Variant = answers.call(method)
		print(method, " answered ", type_string(typeof(answer)), " ", answer)
