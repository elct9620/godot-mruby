extends Node


# Loads a pack carrying a file under a test directory into the running game,
# then lets Ruby name that file's constant: the file reaches the game however
# its export treated test directories. The file's source and the pack's path
# come after `--`. Nothing enters the realm before the pack is loaded.
func _ready() -> void:
	var args := OS.get_cmdline_user_args()
	var packer := PCKPacker.new()
	packer.pck_start(args[1])
	packer.add_file("res://test/leftover.rb", args[0])
	packer.flush()
	ProjectSettings.load_resource_pack(args[1])
	var seeker := Node.new()
	seeker.set_script(load("res://verify/export/seeker.rb"))
	add_child(seeker)
