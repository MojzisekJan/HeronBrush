extends SceneTree
## Checks the demo level still has one of everything, and runs. Delete an entity
## while rearranging the map and this says so.

const EXPECTED := [
	"info_player_start", "worldspawn", "light", "light_sun",
	"fps_weapon_pickup", "fps_pickup",
	"func_door", "trigger_multiple", "func_button", "fps_use_button",
	"fps_key", "fps_key_door", "fps_portal",
	"fps_jump_pad", "fps_ladder", "fps_lift", "fps_water",
	"fps_hurt", "fps_barrel", "fps_enemy",
	"func_rotate", "func_shuttle",
	"fps_checkpoint", "fps_spawner", "trigger_once",
	"fps_smoke", "fps_particles", "fps_cloud", "prop_static", "fps_exit",
]

var failures := 0
var main: Node = null
var frames := 0

func check(ok: bool, label: String) -> void:
	print(("  PASS  " if ok else "  FAIL  ") + label)
	if not ok:
		failures += 1

func _initialize() -> void:
	main = (load("res://scenes/play.tscn") as PackedScene).instantiate()
	main.fallback_level = load("res://maps/demo.tscn")
	get_root().add_child(main)

func _process(_delta: float) -> bool:
	frames += 1
	if frames < 5:
		return false

	var found := {}
	var file := FileAccess.open("res://maps/demo.map", FileAccess.READ)
	check(file != null, "the demo map is there")
	while file != null and not file.eof_reached():
		var line := file.get_line().strip_edges()
		if line.begins_with("\"classname\""):
			var parts := line.split("\" \"")
			if parts.size() > 1:
				found[parts[1].trim_suffix("\"")] = true

	var missing: Array[String] = []
	for classname in EXPECTED:
		if not found.has(classname):
			missing.append(classname)
	check(missing.is_empty(), "it shows all %d kinds of entity, missing %s"
		% [EXPECTED.size(), "nothing" if missing.is_empty() else str(missing)])

	var level: Node = main.get_node_or_null("Level")
	check(level != null and level.get_node_or_null("worldspawn") != null, "and it loads")
	var player: CharacterBody3D = main.get_node("Player")
	check(player.global_position.y < 50.0,
		"with the player on its spawn point, %v" % player.global_position)
	check(get_nodes_in_group("barrels").size() >= 3, "the barrels are in the level")
	check(get_nodes_in_group("ladders").size() >= 1, "so is the ladder")
	check(get_nodes_in_group("exits").size() >= 1, "and the way out")

	print("")
	print("ALL CHECKS PASSED" if failures == 0 else "%d CHECKS FAILED" % failures)
	main.free()
	quit(failures)
	return true
