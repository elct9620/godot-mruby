@tool
extends EditorPlugin
## Creates a Ruby script from the template the script dialog picks for
## CharacterBody2D, under a namespace, as a person would from the editor.
## The check copies it into a copy of the project and reads the file made.

const PATH := "res://enemies/ships/hero_ship.rb"
const CREATE_AT := 30

var frames := 0


func _process(_delta: float) -> void:
	frames += 1
	if frames != CREATE_AT:
		return
	DirAccess.make_dir_recursive_absolute(PATH.get_base_dir())
	var dialog := ScriptCreateDialog.new()
	EditorInterface.get_base_control().add_child(dialog)
	dialog.script_created.connect(_on_script_created)
	var languages: OptionButton = dialog.find_children("*", "OptionButton", true, false)[0]
	for index in languages.item_count:
		if languages.get_item_text(index) == "Ruby":
			languages.select(index)
			languages.item_selected.emit(index)
	dialog.config("Node2D", PATH)
	dialog.popup_centered()
	for menu: OptionButton in dialog.find_children("*", "OptionButton", true, false):
		for index in menu.item_count:
			print("template listed: ", menu.get_item_text(index))
	dialog.config("CharacterBody2D", PATH)
	dialog.popup_centered()
	dialog.get_ok_button().pressed.emit()


func _on_script_created(script: Script) -> void:
	print("template made: ", script.resource_path)
