extends Node


# Reads a node script's exported property from outside Ruby before anything
# builds its Ruby object, has Ruby read the same property, and reads a node
# the scene set nothing on: a read from inside the realm would build the
# object first.
func _ready() -> void:
	print("staged range: ", $Turret.get("range"))
	$Turret.aim()
	print("built range: ", $Turret.get("range"))
	print("default range: ", $Plain.get("range"))
