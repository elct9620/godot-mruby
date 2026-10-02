extends Node2D
## The GDScript twin of src/ops.rb, call for call.

const N := 100000


func _ready() -> void:
	var v := Vector2(1, 2)
	var w := Vector2(3, 4)
	var started := Time.get_ticks_usec()
	for i in N:
		pass
	report("loop", started)
	started = Time.get_ticks_usec()
	for i in N:
		noop()
	report("ruby_call", started)
	started = Time.get_ticks_usec()
	for i in N:
		get_child_count()
	report("engine_call", started)
	started = Time.get_ticks_usec()
	for i in N:
		var at := position
	report("position_get", started)
	started = Time.get_ticks_usec()
	for i in N:
		position = v
	report("position_set", started)
	started = Time.get_ticks_usec()
	for i in N:
		var built := Vector2(1, 2)
	report("vector_new", started)
	started = Time.get_ticks_usec()
	for i in N:
		var sum := v + w
	report("vector_add", started)
	started = Time.get_ticks_usec()
	for i in N:
		var x := v.x
	report("vector_x", started)
	get_tree().quit()


func noop() -> void:
	pass


func report(name: String, started: int) -> void:
	print("op %s %s" % [name, (Time.get_ticks_usec() - started) * 1000.0 / N])
