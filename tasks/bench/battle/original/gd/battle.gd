extends Node2D
## The GDScript twin of src/battle.rb, line for line, for the bench to time
## against it.

signal finished

const Enemy := preload("res://gd/enemy.gd")

@export var wave := 4
@export var spacing := 40.0

var _breached := false
var _finished := false


func _ready() -> void:
	var goal: float = $Base.position.x
	for index in wave:
		var enemy := Enemy.new()
		enemy.position = Vector2(-spacing * index, 0)
		enemy.goal = goal
		enemy.arrived.connect(_breach)
		add_child(enemy)


func _notification(what: int) -> void:
	if what == NOTIFICATION_CHILD_ORDER_CHANGED and is_inside_tree() and enemies().is_empty():
		_finish()


func enemies() -> Array:
	return get_children().filter(
		func(child): return child.get_script() == Enemy and not child.is_queued_for_deletion()
	)


func breached() -> bool:
	return _breached


func _breach() -> void:
	_breached = true
	_finish()


func _finish() -> void:
	if _finished:
		return
	_finished = true
	finished.emit()
