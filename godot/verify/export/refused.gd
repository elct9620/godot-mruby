extends Node


# Adds a test runner node, which an exported game no longer ships in a scene,
# to the running game.
func _ready() -> void:
	add_child(ClassDB.instantiate("RubyTestRunner"))
