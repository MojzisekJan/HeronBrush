extends Node3D
## Runs the arena that heron_brush built from maps/arena.map.
##
## Everything else comes out of the map. This does the two things a text
## scene cannot: drop the player on the spawn, and tell each enemy who to hunt.

@onready var player: CharacterBody3D = $Player
@onready var collected_label: Label = $HUD/Collected
@onready var use_hint: Label = $HUD/UseHint
@onready var message_label: Label = $HUD/Message

## How far you can reach a panel from, in metres.
const USE_RANGE := 3.0

## Quake's light styles, letter for letter. One letter per tenth of a second,
## `a` dark and `m` the light's own brightness, so this only ever scales.
const LIGHT_STYLES := {
	1: "mmnmmommommnonmmonqnmmo",
	2: "mmmmmaaaaammmmmaaaaaabcdefgabcdefg",
	3: "abcdefghijklmnopqrrqponmlkjihgfedcba",
	4: "mamamamamama",
	5: "mmnnmmnnnmmnn",
	6: "abcdefghijklmnopqrstuvwxyzyxwvutsrqponmlkjihgfedcba",
}

## Lights that do not sit still: the node, its pattern and what its energy was
## before anything touched it.
var _styled: Array = []
var _light_time := 0.0

var score := 0

## Where the player comes back after dying, and what they are carrying.
var last_checkpoint: Node3D = null
var keys: Dictionary = {}
## Story flags a script can set, and what was on offer at the start, so a gate
## can ask how many of a kind have been taken.
var flags: Dictionary = {}
var on_offer: Dictionary = {}


func _ready() -> void:
	player.add_to_group("player")
	_apply_environment()
	_place_player()
	_wire_enemies()
	_wire_pickups()
	_wire_checkpoints()
	_wire_keys()
	_wire_lights()


## Sky, fog and ambient light, as set on worldspawn in the map, so two
## levels in one game can look nothing like each other. The property names are
## Godot's own.
func _apply_environment() -> void:
	var holder: WorldEnvironment = get_node_or_null("WorldEnvironment")
	if holder == null or holder.environment == null:
		return
	var world: Node = find_child("worldspawn", true, false)
	if world == null:
		return
	var props: Dictionary = world.get_meta("properties", {})
	if props.is_empty():
		return

	var env: Environment = holder.environment.duplicate(true)
	if props.get("ambient_light_color") is Color:
		env.ambient_light_color = props["ambient_light_color"]
	if props.has("ambient_light_energy"):
		env.ambient_light_energy = float(props["ambient_light_energy"])

	env.fog_enabled = bool(int(props.get("fog_enabled", 0)))
	if env.fog_enabled:
		if props.get("fog_light_color") is Color:
			env.fog_light_color = props["fog_light_color"]
		env.fog_density = float(props.get("fog_density", env.fog_density))

	# The kit ships a photograph of a sky, and a painted sky does not sit next to
	# a photographed one, so two colours are a choice rather than the default.
	if int(props.get("sky", 0)) == 1:
		var painted := ProceduralSkyMaterial.new()
		if props.get("sky_top_color") is Color:
			painted.sky_top_color = props["sky_top_color"]
		if props.get("sky_horizon_color") is Color:
			painted.sky_horizon_color = props["sky_horizon_color"]
			painted.ground_horizon_color = props["sky_horizon_color"]
		var sky := Sky.new()
		sky.sky_material = painted
		env.sky = sky
		env.background_mode = Environment.BG_SKY
	holder.environment = env


## Called by fps_exit when the player walks out. A scene with one level built
## in has nowhere to go, so it starts again; play.tscn overrides this.
func finish_level(next_map: String = "", message: String = "Level done") -> void:
	say(message if not message.is_empty() else "Level done")
	print("arena: level finished, next `%s`" % next_map)
	if next_map.is_empty():
		return
	push_warning("this scene has its level built in, so `%s` cannot be loaded here"
		% next_map)


## Collects the lights the map asked to flicker. No script goes on the light
## nodes: heron_brush writes light_energy and style, and the level drives them.
func _wire_lights() -> void:
	_styled.clear()
	for light in get_tree().get_nodes_in_group("lights"):
		var props: Dictionary = light.get_meta("properties", {})
		var style := int(props.get("style", 0))
		if not LIGHT_STYLES.has(style):
			continue
		_styled.append({"light": light, "pattern": LIGHT_STYLES[style],
			"energy": light.light_energy})
	if not _styled.is_empty():
		print("arena: %d lights flicker" % _styled.size())


## Ten letters a second, no smoothing between them. The steps are the point:
## smooth it out and a failing fluorescent becomes a lava lamp.
func _drive_lights(delta: float) -> void:
	if _styled.is_empty():
		return
	_light_time += delta
	for entry in _styled:
		var light: Light3D = entry["light"]
		if not is_instance_valid(light):
			continue
		var pattern: String = entry["pattern"]
		var at := int(_light_time * 10.0) % pattern.length()
		# `a` is 0 and `m` is 1, so a pattern never brightens a light past what
		# the map asked for unless it uses letters after `m`.
		var level := float(pattern.unicode_at(at) - 97) / 12.0
		light.light_energy = entry["energy"] * level


## Which level is being played, as a res:// path. A scene with its level built
## in knows only its own.
func current_level_path() -> String:
	return scene_file_path


## Writes down enough to come back to: which level, and what the player carries
## into it. Not where every crate ended up.
func save_game() -> void:
	SaveGame.write({
		"level": current_level_path(),
		"health": player.health,
		"score": score,
		"keys": keys.keys(),
	})
	say("Saved")


## Puts a saved game back on, once the level it names is already loaded.
func apply_state(state: Dictionary) -> void:
	if state.is_empty():
		return
	player.health = int(state.get("health", player.health))
	player.health_updated.emit(player.health)
	score = int(state.get("score", 0))
	keys.clear()
	for key in state.get("keys", []):
		keys[str(key)] = true
	_show_score()


## Puts a line on the screen for a few seconds.
func say(text: String, seconds: float = 3.0) -> void:
	if message_label == null:
		return
	message_label.text = text
	message_label.visible = true
	var timer := get_tree().create_timer(seconds)
	timer.timeout.connect(func():
		if message_label != null and message_label.text == text:
			message_label.visible = false)



## info_player_start joins the `spawn_points` group, which the FGD sets up.
##
## The Player node is parked far above the level, so that on the frame before
## this runs it cannot collect whatever pickup sits near the origin.
func _place_player() -> void:
	var spawns := get_tree().get_nodes_in_group("spawn_points")
	if spawns.is_empty():
		push_error("no info_player_start in the map, so the player is left parked above it")
		return
	var spawn := spawns[0] as Node3D
	player.global_position = spawn.global_position
	# The kit's player drives its yaw from rotation_target, so setting rotation
	# alone would be undone by the first mouse movement.
	player.rotation.y = spawn.global_rotation.y
	player.rotation_target.y = spawn.global_rotation.y


## The map cannot fill in an enemy's `player`, so it happens here. Their
## `_process` runs after every `_ready`, so nothing sees a null.
func _wire_enemies() -> void:
	var enemies := get_tree().get_nodes_in_group("enemies")
	for enemy in enemies:
		if "player" in enemy:
			enemy.player = player
	print("arena: %d enemies wired, spawn at %v" % [enemies.size(), player.global_position])


## Pickups come out of the map as instanced scenes, each carrying its own
## worth. They report in here so one counter serves the whole level.
func _wire_pickups() -> void:
	var pickups := get_tree().get_nodes_in_group("collectibles")
	# Counted before any of them are gone.
	on_offer.clear()
	on_offer[""] = pickups.size()
	for pickup in pickups:
		var kind := str(pickup.get_meta("properties", {}).get("kind", ""))
		if not kind.is_empty():
			on_offer[kind] = int(on_offer.get(kind, 0)) + 1
	for pickup in pickups:
		pickup.collected.connect(_on_collected)
	_show_score()
	print("arena: %d pickups worth %d in total" % [pickups.size(), _total_on_offer()])


## Panels marked `usable` are worked by looking at them and pressing Use, which
## is the difference between a wall switch and a floor pad.
func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("use"):
		var panel := _panel_in_reach()
		if panel != null:
			_work(panel)


func _process(delta: float) -> void:
	use_hint.visible = _panel_in_reach() != null
	_drive_lights(delta)


## Whatever usable panel the player is looking at, if any.
func _panel_in_reach() -> Node:
	var camera: Camera3D = player.get_node_or_null("Head/Camera")
	if camera == null:
		return null
	var from: Vector3 = camera.global_position
	var query := PhysicsRayQueryParameters3D.create(from, from - camera.global_basis.z * USE_RANGE)
	# Panels are Area3D, which a ray ignores unless asked not to.
	query.collide_with_areas = true
	query.exclude = [player.get_rid()]

	var hit: Dictionary = get_world_3d().direct_space_state.intersect_ray(query)
	if hit.is_empty():
		return null
	var node: Node = hit["collider"]
	return node if node.is_in_group("usable") else null


## Passes the press on to whatever the panel targets, using the NodePaths heron_brush
## resolved out of the map.
func _work(panel: Node) -> void:
	for path in panel.get_meta("target_paths", []):
		var target: Node = panel.get_node_or_null(path)
		if target != null and target.has_method("_on_triggered"):
			target._on_triggered()


func _on_collected(value: int) -> void:
	score += value
	_show_score()
	_check_gates()


func _show_score() -> void:
	var left := get_tree().get_nodes_in_group("collectibles").size()
	collected_label.text = "%d  (%d left)" % [score, left]


func _total_on_offer() -> int:
	var total := 0
	for pickup in get_tree().get_nodes_in_group("collectibles"):
		total += pickup.value
	return total


## Checkpoints and keys are both just things in the map that report in here, so
## the level keeps the state rather than every entity keeping its own.
func _wire_checkpoints() -> void:
	for point in get_tree().get_nodes_in_group("checkpoints"):
		point.reached.connect(_on_checkpoint)


func _on_checkpoint(point: Node3D) -> void:
	last_checkpoint = point
	print("arena: checkpoint at %v" % point.global_position)


func _wire_keys() -> void:
	for key in get_tree().get_nodes_in_group("keys"):
		key.taken.connect(_on_key_taken)


func _on_key_taken(key_name: String) -> void:
	keys[key_name] = true
	print("arena: picked up the %s key" % key_name)
	_check_gates()


## Asked by doors that are locked.
func has_key(key_name: String) -> bool:
	return keys.get(key_name, false)


## Story flags. A script following the story sets one and every gate waiting on
## it opens, which is the general way to say "this happens after that".
func set_flag(flag: String) -> void:
	flags[flag] = true
	_check_gates()


func has_flag(flag: String) -> bool:
	return flags.get(flag, false)


## How many pickups of a kind have been taken. An empty kind counts them all.
func count_taken(kind: String = "") -> int:
	var left := 0
	for pickup in get_tree().get_nodes_in_group("collectibles"):
		if kind.is_empty() or str(pickup.get_meta("properties", {}).get("kind", "")) == kind:
			left += 1
	return int(on_offer.get(kind, 0)) - left


## Sets off whatever carries this name in the map, wherever it is. This is what
## a story script calls: level.fire("open_the_church").
func fire(target_name: String) -> void:
	for node in _every_node():
		if str(node.get_meta("properties", {}).get("targetname", "")) != target_name:
			continue
		if node.has_method("_on_triggered"):
			node._on_triggered()


func _check_gates() -> void:
	for gate in get_tree().get_nodes_in_group("gates"):
		gate.check(self)


func _every_node() -> Array:
	var out: Array = []
	var stack: Array = [self]
	while not stack.is_empty():
		var node: Node = stack.pop_back()
		for child in node.get_children():
			stack.append(child)
		out.append(node)
	return out


## Called by the player instead of reloading the whole scene, so a run is not
## thrown away by one bad landing.
func respawn_player(who: CharacterBody3D) -> void:
	var spot: Vector3 = _spawn_position()
	if last_checkpoint != null:
		spot = last_checkpoint.global_position + Vector3(0, 1.0, 0)
	who.global_position = spot
	who.velocity = Vector3.ZERO
	who.gravity = 0.0
	who.health = 100
	who.health_updated.emit(who.health)
	print("arena: respawned at %v" % spot)


func _spawn_position() -> Vector3:
	var spawns := get_tree().get_nodes_in_group("spawn_points")
	return (spawns[0] as Node3D).global_position if not spawns.is_empty() else Vector3.ZERO


## Counts what came out of the map. Used by verify_arena.gd.
func census() -> Dictionary:
	return {
		"enemies": get_tree().get_nodes_in_group("enemies").size(),
		"spawn_points": get_tree().get_nodes_in_group("spawn_points").size(),
		"lights": get_tree().get_nodes_in_group("lights").size(),
		"pickups": get_tree().get_nodes_in_group("pickups").size(),
		"decoration": get_tree().get_nodes_in_group("decoration").size(),
		"collectibles": get_tree().get_nodes_in_group("collectibles").size(),
		"checkpoints": get_tree().get_nodes_in_group("checkpoints").size(),
		"keys": get_tree().get_nodes_in_group("keys").size(),
	}
