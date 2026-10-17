extends Node2D
## The GDScript twin of src/turret.rb, line for line.

const RELOAD_TIME := 0.1

@export var mode := &"nearest"
@export var range := 300.0

var _cooldown := 0.0


func _physics_process(delta: float) -> void:
	_cooldown -= delta
	if _cooldown > 0.0:
		return
	var target = _find_target()
	if target == null:
		return
	target.hit()
	_cooldown = RELOAD_TIME


func _find_target():
	var candidates: Array = get_parent().enemies().filter(
		func(enemy): return _measure_distance(enemy) <= range
	)
	if candidates.is_empty():
		return null
	return candidates.reduce(
		func(near, enemy):
			return enemy if _measure_distance(enemy) < _measure_distance(near) else near
	)


func _measure_distance(enemy: Node2D) -> float:
	return position.distance_to(enemy.position)
