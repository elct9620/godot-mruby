@tool
extends EditorPlugin
## Opens a Ruby script in the script editor, as a person would, and prints the
## highlighter the editor chose for it and whether its `def` takes the
## theme's keyword colour, then again once that line is replaced by an
## assignment. The check copies it into a copy of the project.

const PATH := "res://addons/probe/speed.rb"
const OPEN_AT := 30
const LOOK_AT := 40

var frames := 0


func _process(_delta: float) -> void:
	frames += 1
	if frames == OPEN_AT:
		EditorInterface.edit_script(load(PATH))
	elif frames == LOOK_AT:
		_look()


func _look() -> void:
	var editor := EditorInterface.get_script_editor().get_current_editor()
	var highlighter := (editor.get_base_editor() as CodeEdit).syntax_highlighter
	print("highlighter chosen: ", highlighter.get_class())
	var settings := EditorInterface.get_editor_settings()
	var keyword: Color = settings.get_setting("text_editor/theme/highlighting/keyword_color")
	print("keyword coloured: ", _first_color(highlighter) == keyword)
	(editor.get_base_editor() as CodeEdit).set_line(1, "speed = 1")
	print("edited keyword coloured: ", _first_color(highlighter) == keyword)


func _first_color(highlighter: SyntaxHighlighter) -> Variant:
	var line: Dictionary = highlighter.get_line_syntax_highlighting(1)
	return line.get(0, {}).get("color")
