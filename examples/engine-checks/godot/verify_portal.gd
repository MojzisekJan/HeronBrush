extends SceneTree
## Checks that walking into a portal puts the player at the one it targets, and
## that they do not bounce straight back.

var failures := 0
var main: Node = null
var frames := 0
var stage := 0
var player: CharacterBody3D = null
var here: Area3D = null
var there: Area3D = null
var start_pos := Vector3.ZERO


func check(ok: bool, label: String) -> void:
	print(("  PASS  " if ok else "  FAIL  ") + label)
	if not ok:
		failures += 1


func _initialize() -> void:
	var packed: PackedScene = load("res://scenes/play.tscn")
	main = packed.instantiate()
	main.fallback_level = load("res://maps/demo.tscn")
	get_root().add_child(main)


func _process(_delta: float) -> bool:
	frames += 1
	if frames < 3:
		return false

	if stage == 0:
		player = main.get_node("Player")
		var portals := get_nodes_in_group("portals")
		check(portals.size() == 2, "the map placed %d portals" % portals.size())

		for p in portals:
			var props: Dictionary = p.get_meta("properties", {})
			if props.get("targetname", "") == "portal_low":
				here = p
			else:
				there = p
		check(here != null and there != null, "both portals came through named")
		check(here.destination == there, "the ground portal found its far end from the map")
		check(there.destination == here, "and the link works the other way too")
		print("  ground at %v, high at %v" % [here.global_position, there.global_position])

		# Walk in.
		start_pos = here.global_position
		player.global_position = start_pos + Vector3(0, 0.9, 0)
		player.velocity = Vector3(0, 0, 3)
		stage = 1
		return false

	if stage == 1 and frames > 8:
		var moved: float = player.global_position.distance_to(there.global_position)
		check(moved < 3.0, "the player came out at the far portal, %.2f away" % moved)
		check(player.global_position.distance_to(start_pos) > 5.0,
			"and is nowhere near where they went in")
		check(abs(player.rotation.y - there.global_rotation.y) < 0.01,
			"facing the way the far portal faces")
		check(abs(player.rotation_target.y - there.global_rotation.y) < 0.01,
			"and the look direction was carried over, not just the body")
		stage = 2
		return false

	if stage == 2 and frames > 60:
		# Standing in the far portal must not send them back and forth.
		var d: float = player.global_position.distance_to(there.global_position)
		check(d < 4.0, "a second later they are still at the far end, %.2f away" % d)
		check(player.global_position.distance_to(start_pos) > 5.0,
			"so it did not bounce them back")

		print("")
		print("ALL CHECKS PASSED" if failures == 0 else "%d CHECKS FAILED" % failures)
		main.free()
		quit(failures)
		return true

	return false
