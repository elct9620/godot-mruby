extends Node2D
## The GDScript twin of the recommended src/battle.rb, line for line.

signal finished

const Enemy := preload("res://gd/enemy.gd")

@export var wave := 4
@export var spacing := 40.0

var _breached := false
var _finished := false
var _roster: Array = []


func _ready() -> void:
	var goal: float = $Base.position.x
	for index in wave:
		var enemy := Enemy.new()
		enemy.position = Vector2(-spacing * index, 0)
		enemy.goal = goal
		_roster.append(enemy)
		add_child(enemy)
	process_physics_priority = 1


func _physics_process(delta: float) -> void:
	for enemy in _roster.duplicate():
		if enemy.dead:
			_roster.erase(enemy)
			continue
		if enemy.advance(delta):
			continue
		_roster.erase(enemy)
		enemy.queue_free()
		_breach()


func _notification(what: int) -> void:
	if what == NOTIFICATION_CHILD_ORDER_CHANGED and is_inside_tree() and enemies().is_empty():
		_finish()


func enemies() -> Array:
	return _roster.filter(func(enemy): return not enemy.dead)


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
