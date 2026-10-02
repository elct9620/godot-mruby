extends Node
## Plays a battle scene round after round and prints the physics frames the
## rounds took, their wall time, and how the last round ended. This node is
## GDScript so that timing adds no Ruby to either language's battle.

var scene: PackedScene
var wave := 0
var rounds := 0
var played := 0
var frames := 0
var started := 0
var battle: Node


func _ready() -> void:
	var args := OS.get_cmdline_user_args()
	scene = load(args[0])
	wave = int(args[1])
	rounds = int(args[2])
	started = Time.get_ticks_usec()
	_play()


func _physics_process(_delta: float) -> void:
	frames += 1


func _play() -> void:
	battle = scene.instantiate()
	battle.wave = wave
	battle.connect("finished", _on_finished)
	add_child(battle)


func _on_finished() -> void:
	played += 1
	var breached: bool = battle.call("breached") == true
	battle.queue_free()
	if played < rounds:
		_play.call_deferred()
		return
	print(
		"battle frames=%d usec=%d breached=%s" % [frames, Time.get_ticks_usec() - started, breached]
	)
	get_tree().quit()
