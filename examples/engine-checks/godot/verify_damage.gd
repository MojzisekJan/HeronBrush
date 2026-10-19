extends SceneTree
## Checks that damage reaches the health label and that an enemy shooting the
## player costs health. The connection carrying `health_updated` to the HUD was
## lost once already, and nothing on screen said so.

var failures := 0
var main: Node = null
var frames := 0
var stage := 0
var player: CharacterBody3D = null
var health_label: Label = null
var enemy: Node3D = null
var health_before := 0


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
		health_label = main.get_node("HUD/Health")
		check(player.health == 100, "the player starts on 100")
		check(health_label.text == "100%", "and the label agrees: %s" % health_label.text)

		# The signal has to reach the HUD, not just fire.
		check(player.health_updated.is_connected(main.get_node("HUD")._on_health_updated),
			"health_updated is wired to the HUD")

		player.damage(5)
		stage = 1
		return false

	if stage == 1:
		check(player.health == 95, "damage took health to %d" % player.health)
		check(health_label.text == "95%", "and the label followed: %s" % health_label.text)

		# Now the real path: an enemy in range, aiming, firing its timer.
		enemy = get_nodes_in_group("enemies")[0]
		player.global_position = enemy.global_position + Vector3(0, -1.0, 2.0)
		health_before = player.health
		stage = 2
		return false

	if stage == 2 and frames > 20:
		# The enemy turns to face the player in _process, so by now it is aimed.
		enemy._on_timer_timeout()
		stage = 3
		return false

	if stage == 3:
		var lost: int = health_before - player.health
		check(lost > 0, "the enemy's shot cost %d health" % lost)
		check(health_label.text == "%d%%" % player.health,
			"and the label shows it: %s" % health_label.text)

		print("")
		print("ALL CHECKS PASSED" if failures == 0 else "%d CHECKS FAILED" % failures)
		main.free()
		quit(failures)
		return true

	return false
