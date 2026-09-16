extends Node


# Gives Godot another source for held.rb before any node runs it, so what the
# scene prints tells which source the extension ran.
func _ready() -> void:
	var script: Script = load("res://verify/script/held.rb")
	var class_statement := "class Verify::Script::Held < Godot::Node"
	script.source_code = (
		'%s; puts "held.rb as Godot holds it"; def _ready; end; end' % class_statement
	)
	var node := Node.new()
	node.set_script(script)
	add_child(node)
