extends Node2D
## The GDScript twin of the recommended src/enemy.rb, line for line.

signal arrived

@export var speed := 200.0
@export var health := 3

var goal := 0.0
var dead := false
var _x := 0.0
var _y := 0.0


func _ready() -> void:
	var mark := ColorRect.new()
	mark.color = Color(0.9, 0.2, 0.2)
	mark.position = Vector2(-8, -8)
	mark.size = Vector2(16, 16)
	add_child(mark)
	_x = position.x
	_y = position.y


func advance(delta: float) -> bool:
	_x += speed * delta
	position = Vector2(_x, _y)
	return _x < goal


func hit() -> void:
	health -= 1
	if health > 0:
		return
	dead = true
	queue_free()
