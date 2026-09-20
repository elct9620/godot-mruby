extends Node


# Asks node scripts what Godot asks before their files run, and prints the
# answers. The node never enters the tree and loses its script before it is
# freed, so nothing else asks the script anything.
func _ready() -> void:
	var script: Script = load("res://verify/script/header/reported.rb")
	print("instance base type: ", script.get_instance_base_type())
	var node := Node2D.new()
	node.set_script(script)
	print("has _ready: ", node.has_method("_ready"))
	node.set_script(null)
	node.free()
	var marked: Script = load("res://verify/script/header/marked.rb")
	print("is tool: ", marked.is_tool())
	print("is abstract: ", marked.is_abstract())
	print("has wailed: ", $Siren.has_signal("wailed"))
	print("wailed connected: ", $Siren.is_connected("wailed", _on_wailed))
	print("volume: ", $Siren.get("volume"))
	print("volume listed: ", $Siren.get_property_list().any(func(p): return p.name == "volume"))
	print("heard at: ", $Siren.get("heard_at"))
	print("wail listed: ", listed($Siren, "wail"))
	var boss: Script = load("res://verify/script/inherit/boss.rb")
	print("inherited base type: ", boss.get_instance_base_type())
	print("base script: ", boss.get_base_script().resource_path)
	var inheriting := Node2D.new()
	inheriting.set_script(boss)
	print("inherited method listed: ", listed(inheriting, "charge"))
	inheriting.set_script(null)
	inheriting.free()


# What the scene connected the declared signal to; the file never runs, so
# nothing ever emits it.
func _on_wailed(_times: int) -> void:
	pass


# Whether a node answers that it has the method of that name among the ones
# it lists.
func listed(node: Node, method: String) -> bool:
	return node.get_method_list().any(
		func(described: Dictionary) -> bool: return described.name == method
	)
