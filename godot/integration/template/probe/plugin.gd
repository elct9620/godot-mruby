@tool
extends EditorPlugin
## Creates a Ruby script from CharacterBody2D's Basic Movement template in the
## script dialog, under a namespace, as a person would from the editor, after
## printing the templates the dialog lists for Node2D. The dialog fills its
## template menu after it pops up, so each step waits a few frames. The check
## copies it into a copy of the project and reads the file made.

const PATH := "res://enemies/ships/hero_ship.rb"
const TEMPLATE := "CharacterBody2D: Basic Movement"
const OPEN_AT := 30
const LIST_AT := 40
const CREATE_AT := 50

var frames := 0
var dialog: ScriptCreateDialog


func _process(_delta: float) -> void:
	frames += 1
	if frames == OPEN_AT:
		_open()
	elif frames == LIST_AT:
		_list()
		dialog.config("CharacterBody2D", PATH)
	elif frames == CREATE_AT:
		_choose(TEMPLATE)
		dialog.get_ok_button().pressed.emit()


func _open() -> void:
	DirAccess.make_dir_recursive_absolute(PATH.get_base_dir())
	dialog = ScriptCreateDialog.new()
	EditorInterface.get_base_control().add_child(dialog)
	dialog.script_created.connect(_on_script_created)
	var languages: OptionButton = _menus()[0]
	for index in languages.item_count:
		if languages.get_item_text(index) == "Ruby":
			languages.select(index)
			languages.item_selected.emit(index)
	dialog.config("Node2D", PATH)
	dialog.popup_centered()


func _list() -> void:
	for menu in _menus():
		for index in menu.item_count:
			print("template listed: ", menu.get_item_text(index))


func _choose(text: String) -> void:
	for menu in _menus():
		for index in menu.item_count:
			if menu.get_item_text(index) == text:
				menu.select(index)
				menu.item_selected.emit(index)


func _menus() -> Array[Node]:
	return dialog.find_children("*", "OptionButton", true, false)


func _on_script_created(script: Script) -> void:
	print("template made: ", script.resource_path)
