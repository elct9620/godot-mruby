# Prints where user:// is for this project, so a check outside Godot can
# write the run file the editor's test panel leaves and see it removed.
extends SceneTree


func _init() -> void:
	print("user_dir=", OS.get_user_data_dir())
	quit()
