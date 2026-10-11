extends Node


# Reads a node script's exported properties from outside Ruby before its file
# runs, and prints the answers: a read from inside the realm builds the node's
# object, which runs the file.
func _ready() -> void:
	print("volume: ", $Siren.get("volume"))
	print("heard at: ", $Siren.get("heard_at"))


# What the scene connected the declared signal to; the file never runs, so
# nothing ever emits it.
func _on_wailed(_times: int) -> void:
	pass
