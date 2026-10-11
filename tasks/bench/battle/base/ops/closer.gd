extends Node
## Closes each physics frame callback.gd times.

var timer: Node


func _physics_process(_delta: float) -> void:
	timer.close_frame()
