extends Node
## Builds a tree of nodes and times walking it in GDScript, before the bench
## times the same walk from Ruby: the gap is what crossing into the engine
## costs Ruby on each call.

const BRANCHES := 40
const LEAVES := 50
const PASSES := 20


func _ready() -> void:
	var tree := Node.new()
	tree.name = "Tree"
	add_child(tree)
	for branch_index in BRANCHES:
		var branch := Node.new()
		tree.add_child(branch)
		for leaf_index in LEAVES:
			branch.add_child(Node.new())
	var started := Time.get_ticks_usec()
	for pass_index in PASSES:
		walk(tree)
	print("bench walk_gdscript ", Time.get_ticks_usec() - started)


func walk(node: Node) -> int:
	var count := 1
	for child in node.get_children():
		count += walk(child)
	return count
