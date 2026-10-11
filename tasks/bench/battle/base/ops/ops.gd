extends Node2D
## The GDScript twin of src/ops.rb, call for call.

const N := 100000


func _ready() -> void:
	var v := Vector2(1, 2)
	var w := Vector2(3, 4)
	var from := v.x
	var to := w.x
	var start := Time.get_ticks_usec()
	for i in N:
		pass
	report("loop", start)
	start = Time.get_ticks_usec()
	for i in N:
		noop()
	report("ruby_call", start)
	start = Time.get_ticks_usec()
	for i in N:
		get_child_count()
	report("engine_call", start)
	start = Time.get_ticks_usec()
	for i in N:
		var at := position
	report("position_get", start)
	start = Time.get_ticks_usec()
	for i in N:
		position = v
	report("position_set", start)
	start = Time.get_ticks_usec()
	for i in N:
		var frames := Engine.get_process_frames()
	report("singleton_call", start)
	start = Time.get_ticks_usec()
	for i in N:
		var blend = lerp(from, to, 0.5)
	report("utility_call", start)
	start = Time.get_ticks_usec()
	for i in N:
		var vector := Vector2(1, 2)
	report("vector_new", start)
	start = Time.get_ticks_usec()
	for i in N:
		var color := Color(1, 0, 0)
	report("color_new", start)
	start = Time.get_ticks_usec()
	for i in N:
		var sum := v + w
	report("vector_add", start)
	start = Time.get_ticks_usec()
	for i in N:
		var x := v.x
	report("vector_x", start)
	get_tree().quit()


func noop() -> void:
	pass


func report(name: String, start: int) -> void:
	print("op %s %s" % [name, (Time.get_ticks_usec() - start) * 1000.0 / N])
