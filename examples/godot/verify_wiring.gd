extends SceneTree
## Checks that the trigger and the button really drive the door.

var failures := 0

func check(ok: bool, label: String) -> void:
	print(("  PASS  " if ok else "  FAIL  ") + label)
	if not ok:
		failures += 1

func _initialize() -> void:
	var packed: PackedScene = load("res://maps/wiring.tscn")
	check(packed != null, "wiring.tscn loads")
	var root_node: Node = packed.instantiate()
	get_root().add_child(root_node)

	var door := root_node.get_node("func_door")
	# A SceneTree script never runs a frame, so _ready has to be nudged.
	door._ready()
	var trigger := root_node.get_node("trigger_multiple") as Area3D
	var button := root_node.get_node("func_button") as Area3D

	check(door.get_script() != null, "the door has its script attached")
	check(door.has_method("_on_triggered"), "the door has _on_triggered")

	var callable := Callable(door, "_on_triggered")
	check(trigger.is_connected("body_entered", callable), "trigger -> door is connected")
	check(button.is_connected("body_entered", callable), "button -> door is connected")

	var paths: Array = trigger.get_meta("target_paths")
	check(paths.size() == 1, "the trigger knows one target")
	check(trigger.get_node(paths[0]) == door, "and the NodePath reaches the door")

	# Drive it: fire the signal the way a player walking in would.
	var closed: Vector3 = door.position
	trigger.body_entered.emit(null)
	for i in 20:
		door._physics_process(0.1)
	var opened: Vector3 = door.position
	check(opened.y > closed.y + 0.5, "the door moved up, %.2f -> %.2f" % [closed.y, opened.y])

	# It should settle back after the wait runs out.
	for i in 200:
		door._physics_process(0.1)
	check(door.position.is_equal_approx(closed), "and closed again on its own")

	print("")
	print("ALL CHECKS PASSED" if failures == 0 else "%d CHECKS FAILED" % failures)
	quit(failures)
