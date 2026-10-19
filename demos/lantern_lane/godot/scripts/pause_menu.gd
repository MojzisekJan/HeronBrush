extends CanvasLayer
## Pause menu and settings, opened with Escape or Start. Modelled on COGITO's
## EasyMenus but written from scratch: theirs is wired to their save slots.

## Where the settings live between runs.
const CONFIG_PATH := "user://options.cfg"
const SECTION := "options"
## What mouse_sensitivity is at a setting of 1.0. It divides the mouse movement,
## so a bigger number means a slower look and the slider has to invert it.
const SENSITIVITY_BASE := 700.0

var volume := 0.8
var mouse_sens := 1.0
var field_of_view := 80.0
var invert_look := false
var fullscreen := false
var vsync := true

@onready var dim: ColorRect = $Dim
@onready var main_page: VBoxContainer = %MainPage
@onready var options_page: VBoxContainer = %OptionsPage
@onready var resume_button: Button = %Resume
@onready var options_button: Button = %Options
@onready var volume_slider: HSlider = %VolumeSlider
@onready var volume_value: Label = %VolumeValue
@onready var sens_slider: HSlider = %SensSlider
@onready var sens_value: Label = %SensValue
@onready var fov_slider: HSlider = %FovSlider
@onready var fov_value: Label = %FovValue
@onready var invert_check: CheckButton = %InvertCheck
@onready var fullscreen_check: CheckButton = %FullscreenCheck
@onready var vsync_check: CheckButton = %VsyncCheck
@onready var fullscreen_note: Label = %FullscreenPad
@onready var save_button: Button = %Save
@onready var load_button: Button = %Load

## Set by the main menu, where resuming and restarting mean nothing.
var standalone := false


func _ready() -> void:
	# Always, not when-paused: the menu has to hear Escape while the game runs.
	process_mode = Node.PROCESS_MODE_ALWAYS
	visible = false

	load_settings()
	apply_settings()
	fill_controls()

	%Resume.pressed.connect(close)
	%Save.pressed.connect(_on_save)
	%Load.pressed.connect(_on_load)
	%Options.pressed.connect(show_options)
	%Restart.pressed.connect(restart)
	%Quit.pressed.connect(func(): get_tree().quit())
	%Back.pressed.connect(show_main)

	volume_slider.value_changed.connect(func(v): volume = v; changed())
	sens_slider.value_changed.connect(func(v): mouse_sens = v; changed())
	fov_slider.value_changed.connect(func(v): field_of_view = v; changed())
	invert_check.toggled.connect(func(v): invert_look = v; changed())
	fullscreen_check.toggled.connect(func(v): fullscreen = v; changed())
	vsync_check.toggled.connect(func(v): vsync = v; changed())


## Keeps the fullscreen switch honest. A window living in the editor's Game tab
## cannot go fullscreen: the request is accepted and nothing happens, so this
## compares what was asked for with what the window is.
func _process(_delta: float) -> void:
	if not visible:
		return
	var refused: bool = (
		fullscreen
		and DisplayServer.get_name() != "headless"
		and DisplayServer.window_get_mode() != DisplayServer.WINDOW_MODE_FULLSCREEN
	)
	fullscreen_note.text = "not available inside the editor" if refused else ""


func _unhandled_input(event: InputEvent) -> void:
	if not event.is_action_pressed("pause"):
		return
	get_viewport().set_input_as_handled()
	if visible:
		close()
	else:
		open()


# Opening and closing

func open() -> void:
	get_tree().paused = true
	visible = true
	show_main()
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE


func close() -> void:
	visible = false
	get_tree().paused = false
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED


func restart() -> void:
	# The tree has to run again before the scene is swapped, or the new one
	# comes up frozen.
	get_tree().paused = false
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
	get_tree().reload_current_scene()


func show_main() -> void:
	main_page.visible = true
	options_page.visible = false
	# Saving needs a game to save, and loading needs something written down.
	var game: Node = _game()
	save_button.visible = not standalone and game != null and game.has_method("save_game")
	load_button.visible = game != null and game.has_method("load_saved_game") and SaveGame.exists()
	%Resume.visible = not standalone
	%Restart.visible = not standalone
	if standalone:
		options_button.grab_focus()
	else:
		resume_button.grab_focus()


func show_options() -> void:
	main_page.visible = false
	options_page.visible = true
	volume_slider.grab_focus()


## Whatever is running the game, if anything is.
func _game() -> Node:
	return get_tree().current_scene


func _on_save() -> void:
	var game: Node = _game()
	if game != null and game.has_method("save_game"):
		game.save_game()
	close()


func _on_load() -> void:
	var game: Node = _game()
	if game != null and game.has_method("load_saved_game") and game.load_saved_game():
		close()


# Settings

func changed() -> void:
	apply_settings()
	fill_labels()
	save_settings()


## Puts the stored values into the controls. Only at startup: doing it on every
## change would fight the slider the player is dragging.
func fill_controls() -> void:
	volume_slider.value = volume
	sens_slider.value = mouse_sens
	fov_slider.value = field_of_view
	invert_check.button_pressed = invert_look
	fullscreen_check.button_pressed = fullscreen
	vsync_check.button_pressed = vsync
	fill_labels()


func fill_labels() -> void:
	volume_value.text = "%d %%" % roundi(volume * 100.0)
	sens_value.text = "%.2f" % mouse_sens
	fov_value.text = "%d" % roundi(field_of_view)


func apply_settings() -> void:
	# Silence is -80 dB rather than a very small number, which would still be
	# faintly audible.
	var db: float = -80.0 if volume <= 0.001 else linear_to_db(volume)
	AudioServer.set_bus_volume_db(AudioServer.get_bus_index("Master"), db)

	var player: Node = get_tree().get_first_node_in_group("player")
	if player != null:
		player.mouse_sensitivity = SENSITIVITY_BASE / max(mouse_sens, 0.05)
		player.invert_look = invert_look
		player.camera.fov = field_of_view

	# A headless run has no window to set modes on, and the checks run headless.
	if DisplayServer.get_name() == "headless":
		return
	DisplayServer.window_set_mode(
		DisplayServer.WINDOW_MODE_FULLSCREEN if fullscreen else DisplayServer.WINDOW_MODE_WINDOWED)
	DisplayServer.window_set_vsync_mode(
		DisplayServer.VSYNC_ENABLED if vsync else DisplayServer.VSYNC_DISABLED)


func load_settings() -> void:
	var config := ConfigFile.new()
	if config.load(CONFIG_PATH) != OK:
		return
	volume = config.get_value(SECTION, "volume", volume)
	mouse_sens = config.get_value(SECTION, "mouse_sens", mouse_sens)
	field_of_view = config.get_value(SECTION, "fov", field_of_view)
	invert_look = config.get_value(SECTION, "invert_look", invert_look)
	fullscreen = config.get_value(SECTION, "fullscreen", fullscreen)
	vsync = config.get_value(SECTION, "vsync", vsync)


func save_settings() -> void:
	var config := ConfigFile.new()
	config.set_value(SECTION, "volume", volume)
	config.set_value(SECTION, "mouse_sens", mouse_sens)
	config.set_value(SECTION, "fov", field_of_view)
	config.set_value(SECTION, "invert_look", invert_look)
	config.set_value(SECTION, "fullscreen", fullscreen)
	config.set_value(SECTION, "vsync", vsync)
	config.save(CONFIG_PATH)
