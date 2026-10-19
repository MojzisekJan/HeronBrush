extends SceneTree
## Checks the two doors in the arena: one opened by walking into a trigger, the
## other by standing on a button on the far side of the level.

var failures := 0
var main: Node = null
var frames := 0
var stage := 0

var player: CharacterBody3D = null
var trigger_door: Node3D = null
var button_door: Node3D = null
var trigger: Area3D = null
var button: Area3D = null
var trigger_shut := Vector3.ZERO
var button_shut := Vector3.ZERO


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
	if frames < 3:
		return false
	var arena := main.get_node("Level")

	if stage == 0:
		player = main.get_node("Player")
		trigger_door = arena.get_node("func_door")
		button_door = arena.get_node("func_door_1")
		trigger = arena.get_node("trigger_multiple")
		button = arena.get_node("func_button")
		trigger_shut = trigger_door.global_position
		button_shut = button_door.global_position

		check(trigger_door.get_script() != null, "the trigger door has its script")
		check(button_door.get_script() != null, "the button door has its script")
		check(trigger.is_connected("body_entered", Callable(trigger_door, "_on_triggered")),
			"the trigger is wired to its own door")
		check(button.is_connected("body_entered", Callable(button_door, "_on_triggered")),
			"the button is wired to the other one")
		check(not button.is_connected("body_entered", Callable(trigger_door, "_on_triggered")),
			"and not to both at once")
		print("  trigger door at %v, button door at %v, button at %v" % [
			trigger_shut, button_shut, button.global_position])

		player.global_position = trigger.global_position
		player.velocity = Vector3.ZERO
		stage = 1
		return false

	if stage == 1 and frames > 60:
		var moved: float = trigger_door.global_position.distance_to(trigger_shut)
		check(moved > 1.0, "walking into the trigger slid its door %.2f units" % moved)
		check(button_door.global_position.distance_to(button_shut) < 0.01,
			"and left the far door shut")

		# Off to the button on the other side of the arena.
		player.global_position = button.global_position + Vector3(0, 1.0, 0)
		player.velocity = Vector3.ZERO
		stage = 2
		return false

	if stage == 2 and frames > 130:
		var moved: float = button_door.global_position.distance_to(button_shut)
		check(moved > 1.0, "standing on the button slid its door %.2f units" % moved)
		stage = 3
		return false

	# The first door waits 3 seconds and then slides back. Watch until it is
	# shut rather than guessing the frame it lands on; the door also stands
	# still while it waits, so "has stopped moving" would fire too early.
	if stage == 3:
		var gap: float = trigger_door.global_position.distance_to(trigger_shut)
		if gap >= 0.01 and frames < 600:
			return false
		check(gap < 0.01, "the first door shut itself again, %.3f from where it started" % gap)
		check(button_door.global_position.distance_to(button_shut) > 1.0,
			"while the second is held open by the player standing on the button")

		print("")
		print("ALL CHECKS PASSED" if failures == 0 else "%d CHECKS FAILED" % failures)
		main.free()
		quit(failures)
		return true

	return false
