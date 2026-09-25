@tool
extends EditorPlugin
## Runs one test directory from the test panel and prints what the panel
## lists, then activates the first row and prints the line the script editor
## opened. The check copies it into a copy of the project, which plays its
## scenes headless.

const DIRECTORY := "res://integration/runner/failing/assertion"
const PRESS_AT := 30
const LOOK_EVERY := 30

var frames := 0
var is_pressed := false
var is_opened := false


func _process(_delta: float) -> void:
	frames += 1
	var panel := _panel(get_tree().root)
	if panel == null:
		return
	if frames == PRESS_AT:
		_choose(panel)
		_button(panel, "Run").pressed.emit()
		is_pressed = true
	elif is_pressed and not is_opened and frames % LOOK_EVERY == 0:
		_look(panel)


func _choose(panel: Node) -> void:
	var directories: OptionButton = panel.find_children("*", "OptionButton", true, false)[0]
	for index in directories.item_count:
		if directories.get_item_text(index) == DIRECTORY:
			directories.select(index)


func _look(panel: Node) -> void:
	var tree: Tree = panel.find_children("*", "Tree", true, false)[0]
	var root := tree.get_root()
	if EditorInterface.is_playing_scene() or root == null or root.get_child_count() == 0:
		return
	var status: Label = panel.find_children("*", "Label", true, false)[0]
	print("panel status: ", status.text)
	for row in root.get_children():
		print("panel row: ", row.get_text(0), " | ", row.get_text(1), " | ", row.get_text(2))
	root.get_child(0).select(0)
	tree.item_activated.emit()
	is_opened = true
	var editor := EditorInterface.get_script_editor()
	var code: CodeEdit = editor.get_current_editor().get_base_editor()
	print(
		"panel opened: ", editor.get_current_script().resource_path, ":", code.get_caret_line() + 1
	)


func _button(panel: Node, text: String) -> Button:
	for button: Button in panel.find_children("*", "Button", true, false):
		if button.text == text:
			return button
	return null


func _panel(node: Node) -> Node:
	if node.get_class() == "RubyTestPanel":
		return node
	for child in node.get_children(true):
		var found := _panel(child)
		if found:
			return found
	return null
