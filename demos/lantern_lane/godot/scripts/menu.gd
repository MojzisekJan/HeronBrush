extends Control
## The main menu. Which levels exist is read off the disk, so a map somebody
## adds turns up here without anybody touching this file.

## Where a new game starts.
@export var first_level: String = "res://maps/lantern_lane.tscn"
## Where levels are looked for, for the list.
@export var maps_folder: String = "res://maps"

@onready var buttons: VBoxContainer = %Buttons
@onready var levels: VBoxContainer = %Levels
@onready var continue_button: Button = %Continue

var _settings: CanvasLayer = null


func _ready() -> void:
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	%NewGame.pressed.connect(func(): start(first_level))
	%Continue.pressed.connect(_on_continue)
	%Levels_Button.pressed.connect(_show_levels)
	%Settings.pressed.connect(_show_settings)
	%Quit.pressed.connect(func(): get_tree().quit())
	%Back.pressed.connect(_show_main)

	continue_button.visible = SaveGame.exists()
	_fill_levels()
	_show_main()


func _show_main() -> void:
	buttons.visible = true
	levels.visible = false
	%NewGame.grab_focus()


func _show_levels() -> void:
	buttons.visible = false
	levels.visible = true


func _show_settings() -> void:
	if _settings == null:
		_settings = preload("res://objects/pause_menu.tscn").instantiate()
		_settings.standalone = true
		add_child(_settings)
	_settings.open()
	_settings.show_options()


## Every built level in the maps folder, newest names last. Levels arrive by
## being built, so the list is the folder rather than anything written down.
func _fill_levels() -> void:
	for child in levels.get_children():
		if child.name != "Back" and child.name != "LevelsTitle":
			child.queue_free()
	var folder := DirAccess.open(maps_folder)
	if folder == null:
		return
	var names := folder.get_files()
	names.sort()
	for file in names:
		if not file.ends_with(".tscn"):
			continue
		var path := "%s/%s" % [maps_folder, file]
		var button := Button.new()
		button.text = file.get_basename()
		button.custom_minimum_size = Vector2(320, 40)
		button.pressed.connect(start.bind(path))
		levels.add_child(button)
		levels.move_child(button, levels.get_child_count() - 2)


func _on_continue() -> void:
	start("", SaveGame.read())


## Swaps this menu for the game, handing the level over before it enters the
## tree. Same door `heron_brush play` uses with `--level=`.
func start(level: String, state: Dictionary = {}) -> void:
	var path := level
	if path.is_empty():
		path = str(state.get("level", first_level))
	if not ResourceLoader.exists(path):
		push_error("menu: no level at %s" % path)
		return

	var game: Node = preload("res://scenes/play.tscn").instantiate()
	game.fallback_level = load(path)
	game.pending_state = state
	get_tree().root.add_child(game)
	get_tree().current_scene = game
	queue_free()
