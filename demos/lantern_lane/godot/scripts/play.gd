extends "res://scripts/arena.gd"
## Plays whichever level it is given, so a new map needs nothing built in Godot.
## `heron_brush play maps/x.map` starts this with `-- --level=res://maps/x.tscn`.

## Used when nothing is named on the command line, so that pressing F5 in the
## editor still gets you somewhere.
@export var fallback_level: PackedScene = preload("res://maps/lantern_lane.tscn")

## Set before this is put in the tree, to carry a saved game into it.
var pending_state: Dictionary = {}
## What is being played now, so a save knows what to write down.
var playing: String = ""


func _ready() -> void:
	var path := level_from_command_line()
	var packed: PackedScene = null
	if path.is_empty():
		packed = fallback_level
	else:
		packed = load(path) as PackedScene
		if packed == null:
			push_error("cannot load the level `%s`, falling back" % path)
			packed = fallback_level

	if packed != null:
		var level := packed.instantiate()
		level.name = "Level"
		# Added before the wiring below runs, so that the spawn point, the
		# enemies and the pickups are already in their groups when it looks.
		add_child(level)
		playing = path if not path.is_empty() else packed.resource_path
		print("play: %s" % playing)
	else:
		push_error("no level to play; pass -- --level=res://maps/your.tscn")

	super._ready()
	if not pending_state.is_empty():
		apply_state(pending_state)
		pending_state = {}


## Which level is on, rather than which scene this is.
func current_level_path() -> String:
	return playing


## Loads the level a save names, then puts the saved state back on top of it.
func load_saved_game() -> bool:
	var state := SaveGame.read()
	var path := str(state.get("level", ""))
	if path.is_empty() or not ResourceLoader.exists(path):
		push_warning("play: nothing saved to go back to")
		return false
	if not swap_level(path):
		return false
	apply_state(state)
	return true


## Swaps the level for the next one. The player, the HUD and the pause menu
## stay as they were, so a campaign runs without a loading screen.
func finish_level(next_map: String = "", message: String = "Level done") -> void:
	say(message if not message.is_empty() else "Level done")
	if next_map.is_empty():
		print("play: level finished, nothing after it")
		return

	if not swap_level(level_path(next_map)):
		return
	# Saved on arriving, not on leaving, so coming back puts you at the start of
	# a level rather than at the exit of the one before it.
	save_game()


## Puts a different level in and wires it up, keeping everything above it.
func swap_level(path: String) -> bool:
	var packed := load(path) as PackedScene
	if packed == null:
		push_error("play: cannot load `%s`" % path)
		return false

	var old := get_node_or_null("Level")
	if old != null:
		# Taken out at once rather than queued, or the wiring below still finds
		# the old level's spawn point and its pickups.
		remove_child(old)
		old.queue_free()

	var level := packed.instantiate()
	level.name = "Level"
	add_child(level)
	playing = path
	print("play: on to %s" % path)

	last_checkpoint = null
	keys.clear()
	_apply_environment()
	_place_player()
	_wire_enemies()
	_wire_pickups()
	_wire_checkpoints()
	_wire_keys()
	_wire_lights()
	return true


## `arena` and `res://maps/lantern_lane.tscn` both mean the same thing, because a level
## designer should not have to write engine paths.
func level_path(name: String) -> String:
	var trimmed := name.strip_edges()
	if trimmed.begins_with("res://"):
		return trimmed
	if trimmed.ends_with(".tscn") or trimmed.ends_with(".map"):
		trimmed = trimmed.get_basename()
	return "res://maps/%s.tscn" % trimmed


## Reads `--level=res://...` from what came after the `--` on the command line.
func level_from_command_line() -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--level="):
			return argument.substr("--level=".length())
	return ""
