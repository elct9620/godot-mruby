extends Node2D
## The GDScript twin of src/turret.rb, line for line.

const RELOAD := 0.1

@export var mode := &"nearest"
@export var range := 300.0

var _cooldown := 0.0


func _physics_process(delta: float) -> void:
	_cooldown -= delta
	if _cooldown > 0.0:
		return
	var target = _nearest()
	if target == null:
		return
	target.hit()
	_cooldown = RELOAD


func _nearest():
	var in_range: Array = get_parent().enemies().filter(
		func(enemy): return _distance(enemy) <= range
	)
	if in_range.is_empty():
		return null
	return in_range.reduce(
		func(near, enemy): return enemy if _distance(enemy) < _distance(near) else near
	)


func _distance(enemy: Node2D) -> float:
	return position.distance_to(enemy.position)
