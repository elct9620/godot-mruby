extends Node2D
## The GDScript twin of src/operations.rb, call for call.

const REPEATS := 100000


func _ready() -> void:
	var v := Vector2(1, 2)
	var w := Vector2(3, 4)
	var from := v.x
	var to := w.x
	var start := Time.get_ticks_usec()
	for i in REPEATS:
		pass
	report("loop", start)
	start = Time.get_ticks_usec()
	for i in REPEATS:
		noop()
	report("ruby_call", start)
	start = Time.get_ticks_usec()
	for i in REPEATS:
		get_child_count()
	report("engine_call", start)
	start = Time.get_ticks_usec()
	for i in REPEATS:
		var at := position
	report("position_get", start)
	start = Time.get_ticks_usec()
	for i in REPEATS:
		position = v
	report("position_set", start)
	start = Time.get_ticks_usec()
	for i in REPEATS:
		position.y = 5.0
	report("member_set", start)
	start = Time.get_ticks_usec()
	for i in REPEATS:
		var frames := Engine.get_process_frames()
	report("singleton_call", start)
	start = Time.get_ticks_usec()
	for i in REPEATS:
		var blend = lerp(from, to, 0.5)
	report("utility_call", start)
	start = Time.get_ticks_usec()
	for i in REPEATS:
		var vector := Vector2(1, 2)
	report("vector_new", start)
	start = Time.get_ticks_usec()
	for i in REPEATS:
		var color := Color(1, 0, 0)
	report("color_new", start)
	start = Time.get_ticks_usec()
	for i in REPEATS:
		var sum := v + w
	report("vector_add", start)
	start = Time.get_ticks_usec()
	for i in REPEATS:
		var x := v.x
	report("vector_x", start)
	get_tree().quit()


func noop() -> void:
	pass


func report(name: String, start: int) -> void:
	print("op %s %s" % [name, (Time.get_ticks_usec() - start) * 1000.0 / REPEATS])
