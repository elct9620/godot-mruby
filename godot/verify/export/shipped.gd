extends Node


# Prints each path after `--` that the running game has a resource at.
func _ready() -> void:
	for path in OS.get_cmdline_user_args():
		if ResourceLoader.exists(path):
			print("shipped: ", path)
