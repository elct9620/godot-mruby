extends Node


# Gives Godot another source for held.rb before any node runs it, so what the
# scene prints tells which source the extension ran.
func _ready() -> void:
	var script: Script = load("res://source/held.rb")
	script.source_code = 'puts "held.rb as Godot holds it"'
	var node := Node.new()
	node.set_script(script)
	add_child(node)
