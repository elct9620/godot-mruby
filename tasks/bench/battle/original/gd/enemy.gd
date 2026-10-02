extends Node2D
## The GDScript twin of src/enemy.rb, line for line.

signal arrived

@export var speed := 200.0
@export var health := 3

var goal := 0.0


func _ready() -> void:
	var mark := ColorRect.new()
	mark.color = Color(0.9, 0.2, 0.2)
	mark.position = Vector2(-8, -8)
	mark.size = Vector2(16, 16)
	add_child(mark)


func _physics_process(delta: float) -> void:
	position = position + Vector2(speed * delta, 0)
	if position.x < goal:
		return
	arrived.emit()
	queue_free()


func hit() -> void:
	health -= 1
	if health <= 0:
		queue_free()
