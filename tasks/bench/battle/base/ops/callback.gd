extends Node
## Adds nodes of the ticker script the command line names, plays physics
## frames, and prints the nanoseconds of one ticker's callback: the time
## between this node's physics callback, which runs first, and the closing
## node's, which runs last. It is GDScript so that timing adds no Ruby.

const TICKERS := 2000
const FRAMES := 100

var frames := 0
var started := 0
var total := 0


func _ready() -> void:
	process_physics_priority = -1000
	var script: Script = load(OS.get_cmdline_user_args()[0])
	for index in TICKERS:
		var ticker := Node.new()
		ticker.set_script(script)
		add_child(ticker)
	var closing := Node.new()
	closing.set_script(load("res://ops/closing.gd"))
	closing.process_physics_priority = 1000
	closing.timer = self
	add_child(closing)


func _physics_process(_delta: float) -> void:
	started = Time.get_ticks_usec()


func close_frame() -> void:
	total += Time.get_ticks_usec() - started
	frames += 1
	if frames == FRAMES:
		print("op callback %s" % (total * 1000.0 / (TICKERS * FRAMES)))
		get_tree().quit()
