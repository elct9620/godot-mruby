extends Node


# Reads a node script's exported property before anything builds its Ruby
# object, then has Ruby read the same property, reads a node the scene set
# nothing on, and writes a property only the node's own class has.
func _ready() -> void:
	print("staged range: ", $Turret.get("range"))
	$Turret.aim()
	print("built range: ", $Turret.get("range"))
	print("default range: ", $Plain.get("range"))
	$Sprite.cover()
	$Sprite.set("centered", false)
	print("centered: ", $Sprite.is_centered())
	$Sprite.cover()
