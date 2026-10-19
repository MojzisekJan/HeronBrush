extends SceneTree
var game: Node3D
func _initialize() -> void:
	call_deferred("run")
func settle(count := 5) -> void:
	for _i in range(count):
		await physics_frame
func group(name: String) -> Array[Node]:
	return get_nodes_in_group(name)
func check(condition: bool, note := "Campaign check failed") -> bool:
	if not condition:
		push_error(note)
		quit(1)
	return condition
func run() -> void:
	if FileAccess.file_exists("user://canal-relay.json"):
		DirAccess.remove_absolute(ProjectSettings.globalize_path("user://canal-relay.json"))
	game = load("res://main.tscn").instantiate()
	root.add_child(game)
	await settle()
	if not check(game.level_name == "courtyard"): return
	if not check(not game.try_exit("relay", "pumphouse")): return
	# Native sensors collect cells, including the stair/balcony pickup.
	for chapter in ["courtyard", "pumphouse", "beacon"]:
		if not check(game.level_name == chapter): return
		var gate := group("relay_doors")[0] as AnimatableBody3D
		var shape := gate.find_children("*", "CollisionShape3D", true, false)[0] as CollisionShape3D
		var centre := Vector3.ZERO
		for point in shape.shape.points: centre += point
		centre = shape.to_global(centre / shape.shape.points.size())
		var beam := PhysicsRayQueryParameters3D.create(centre + Vector3(0, 0, 2), centre - Vector3(0, 0, 2))
		beam.exclude = [game.player.get_rid()]
		var blocked := game.get_world_3d().direct_space_state.intersect_ray(beam)
		if not check(blocked.get("collider") == gate, "Closed gate has native collision"): return
		var expected := 2 if chapter != "pumphouse" else 3
		var cells := group("relay_cells").duplicate()
		for cell in cells:
			game.player.global_position = Vector3(cell.global_position.x, cell.global_position.y - 0.70, cell.global_position.z)
			game.player.velocity = Vector3.ZERO
			await settle()
		if not check(game.energy >= expected): return
		if not check(group("relay_cells").is_empty()): return
		# Falling into the generated canal Area3D returns to spawn, retaining inventory.
		var collected: int = game.energy
		game.player.global_position = Vector3(-5, -0.6, 0)
		game.player.velocity = Vector3.ZERO
		await settle(10)
		var spawn := group("relay_spawns")[0] as Node3D
		if not check(Vector2(game.player.global_position.x, game.player.global_position.z).distance_to(Vector2(spawn.global_position.x, spawn.global_position.z)) < 0.2 and game.energy == collected, "Canal sensor respawns without losing cells"): return
		var station := group("relay_stations")[0] as Node3D
		if not check(not station.active): return
		# Exercise actual F-input, aim test and obstruction ray, not a replacement controller.
		game.player.global_position = station.global_position + Vector3(1.2, 0.05, 0)
		game.player.velocity = Vector3.ZERO
		game.player.camera.look_at(station.global_position + Vector3.UP)
		await settle()
		game.player.camera.look_at(station.global_position + Vector3.UP)
		Input.action_press("use")
		await settle(2)
		Input.action_release("use")
		await settle(60)
		print("Verified relay and gate: ", chapter)
		if not check(station.active): return
		if not check(gate.opened): return
		var cleared := game.get_world_3d().direct_space_state.intersect_ray(beam)
		if not check(cleared.get("collider") != gate, "Powered gate physically clears the crossing"): return
		# Save/restart must keep the powered gate, claimed cells and inventory.
		var energy: int = game.energy
		game.queue_free()
		await settle()
		game = load("res://main.tscn").instantiate()
		root.add_child(game)
		await settle()
		if not check(game.level_name == chapter and game.energy == energy): return
		if not check(group("relay_cells").is_empty()): return
		if not check(group("relay_stations")[0].active and group("relay_doors")[0].opened): return
		# Exit through the generated native Area3D.
		var exit_area: Area3D
		for zone in group("relay_zones"):
			if zone.get_meta("properties", {}).get("kind") == "exit":
				exit_area = zone
		var collision := exit_area.find_children("*", "CollisionShape3D", true, false)[0] as CollisionShape3D
		var exit_centre := Vector3.ZERO
		for point in collision.shape.points:
			exit_centre += point
		exit_centre /= collision.shape.points.size()
		exit_centre = collision.to_global(exit_centre)
		game.player.global_position = Vector3(exit_centre.x, 0.05, exit_centre.z)
		game.player.velocity = Vector3.ZERO
		await settle(10)
	if not check(game.won): return
	if not check(game.energy == 1, "Optional beacon bonus survives the campaign"): return
	print("PASS: three chapters, locked exits, native pickups and F interaction, powered gates, per-chapter save/restart, exit sensors and victory.")
	quit()
