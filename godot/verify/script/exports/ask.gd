extends Node


# Reads a node script's exported property before anything builds its Ruby
# object, then has Ruby read the same property, and reads a node the scene
# set nothing on.
func _ready() -> void:
	print("staged range: ", $Turret.get("range"))
	$Turret.aim()
	print("built range: ", $Turret.get("range"))
	print("default range: ", $Plain.get("range"))
