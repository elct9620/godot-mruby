extends Node


# Asks a node script what Godot asks before its file runs, and prints the
# answers. The node never enters the tree and loses its script before it is
# freed, so nothing else asks the script anything.
func _ready() -> void:
	var script: Script = load("res://verify/script/header/reported.rb")
	print("instance base type: ", script.get_instance_base_type())
	var node := Node2D.new()
	node.set_script(script)
	print("has _ready: ", node.has_method("_ready"))
	node.set_script(null)
	node.free()
