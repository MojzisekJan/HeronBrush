extends SceneTree
## Walks a body into the trigger and then into the button, with real physics
## running, and checks that the door opens both times.
##
## This is the check that catches a collision mask that does not line up. A
## connection can be present and correct and still never fire.

var failures := 0
var main: Node = null
var frames := 0
var stage := 0

var door: Node3D = null
var closed_at := Vector3.ZERO
var body: CharacterBody3D = null
var seen: Dictionary = {}


func check(ok: bool, label: String) -> void:
	print(("  PASS  " if ok else "  FAIL  ") + label)
	if not ok:
		failures += 1


func _initialize() -> void:
	var packed: PackedScene = load("res://maps/wiring.tscn")
	main = packed.instantiate()
	get_root().add_child(main)

	# A stand-in for the player: default collision layer, which is 1.
	body = CharacterBody3D.new()
	var shape := CollisionShape3D.new()
	var capsule := CapsuleShape3D.new()
	capsule.height = 1.6
	capsule.radius = 0.3
	shape.shape = capsule
	body.add_child(shape)
	get_root().add_child(body)


func _watch(area_name: String) -> void:
	var area: Area3D = main.get_node(area_name)
	area.body_entered.connect(func(b): seen[area_name] = b)
	print("  %s at %v, mask %d" % [area_name, area.global_position, area.collision_mask])


func _process(_delta: float) -> bool:
	frames += 1
	if frames < 3:
		return false

	if stage == 0:
		door = main.get_node("func_door")
		closed_at = door.global_position
		_watch("trigger_multiple")
		_watch("func_button")
		check(body.collision_layer == 1, "the stand-in body sits on layer 1")

		# Into the trigger.
		body.global_position = main.get_node("trigger_multiple").global_position
		stage = 1
		return false

	if stage == 1 and frames > 40:
		check(seen.has("trigger_multiple"), "walking into the trigger fired it")
		var risen: float = door.global_position.y - closed_at.y
		check(risen > 0.5, "the door opened %.2f units" % risen)

		# Out of the way, and wait for it to close again.
		body.global_position = Vector3(0, 1, 0)
		stage = 2
		return false

	if stage == 2 and frames > 300:
		check(door.global_position.is_equal_approx(closed_at),
			"the door closed again, y = %.2f" % door.global_position.y)

		# Now the button.
		body.global_position = main.get_node("func_button").global_position + Vector3(0, 0.6, 0)
		stage = 3
		return false

	if stage == 3 and frames > 360:
		check(seen.has("func_button"), "stepping on the button fired it")
		var risen: float = door.global_position.y - closed_at.y
		check(risen > 0.5, "the button opened the door %.2f units" % risen)

		# The check above only means something if the opposite also holds. A
		# body on a layer the trigger does not watch must be ignored, which is
		# how a mismatched collision mask shows up in a real project.
		body.global_position = Vector3(0, 100, 0)
		var stranger := CharacterBody3D.new()
		stranger.collision_layer = 4
		var shape := CollisionShape3D.new()
		var capsule := CapsuleShape3D.new()
		capsule.height = 1.6
		capsule.radius = 0.3
		shape.shape = capsule
		stranger.add_child(shape)
		get_root().add_child(stranger)
		stranger.global_position = main.get_node("trigger_multiple").global_position
		seen.erase("trigger_multiple")
		stage = 4
		return false

	if stage == 4 and frames > 420:
		check(not seen.has("trigger_multiple"),
			"a body on layer 4 is ignored, so the mask really is being honoured")

		print("")
		print("ALL CHECKS PASSED" if failures == 0 else "%d CHECKS FAILED" % failures)
		quit(failures)
		return true

	return false
