extends Node3D
const CHAPTERS := ["courtyard", "pumphouse", "beacon"]
const TITLES := ["The copper courtyard", "The pump house", "The beacon"]
const SAVE := "user://canal-relay.json"
var player: CharacterBody3D
var level: Node3D
var level_name := "courtyard"
var energy := 0
var claimed: Dictionary = {}
var powered: Dictionary = {}
var won := false
var elapsed := 0.0
var message := "Collect cells, face a relay station and press F."
var hud: Label
var loading := false
func _ready() -> void:
	add_to_group("relay_game")
	for entry in [["forward", KEY_W], ["back", KEY_S], ["left", KEY_A], ["right", KEY_D], ["jump", KEY_SPACE], ["use", KEY_F], ["restart", KEY_R]]:
		if not InputMap.has_action(entry[0]):
			InputMap.add_action(entry[0])
			var key := InputEventKey.new()
			key.physical_keycode = entry[1]
			InputMap.action_add_event(entry[0], key)
	var light := DirectionalLight3D.new()
	light.rotation_degrees = Vector3(-55, -35, 0)
	light.light_energy = 1.4
	light.shadow_enabled = true
	add_child(light)
	var environment := WorldEnvironment.new()
	environment.environment = Environment.new()
	environment.environment.background_mode = Environment.BG_COLOR
	environment.environment.background_color = Color(0.52, 0.7, 0.82)
	environment.environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.environment.ambient_light_color = Color(0.8, 0.88, 1)
	environment.environment.ambient_light_energy = 0.65
	add_child(environment)
	player = CharacterBody3D.new()
	player.set_script(load("res://scripts/player.gd"))
	add_child(player)
	var layer := CanvasLayer.new()
	add_child(layer)
	var panel := PanelContainer.new()
	panel.position = Vector2(20, 20)
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.04, 0.1, 0.13, 0.88)
	style.content_margin_left = 16
	style.content_margin_right = 16
	style.content_margin_top = 12
	style.content_margin_bottom = 12
	panel.add_theme_stylebox_override("panel", style)
	layer.add_child(panel)
	hud = Label.new()
	hud.add_theme_font_size_override("font_size", 19)
	panel.add_child(hud)
	read_save()
	load_level(level_name)
	if DisplayServer.get_name() != "headless":
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and event.keycode == KEY_ESCAPE:
		Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	if event is InputEventMouseButton and event.pressed and not won:
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
func _physics_process(dt: float) -> void:
	if not won:
		elapsed += dt
	if Input.is_action_just_pressed("use"):
		use_nearest_station()
	if Input.is_action_just_pressed("restart"):
		energy = 0; claimed.clear(); powered.clear(); won = false; elapsed = 0
		load_level("courtyard")
		save_progress()
	refresh_hud()
func token(id: String) -> String:
	return level_name + "/" + id
func load_level(chapter: String) -> bool:
	if chapter not in CHAPTERS:
		return false
	loading = true
	if is_instance_valid(level):
		remove_child(level)
		level.queue_free()
	level_name = chapter
	level = load("res://maps/%s.tscn" % chapter).instantiate()
	add_child(level)
	for cell in get_tree().get_nodes_in_group("relay_cells"):
		if claimed.has(token(cell.cell_id)):
			cell.queue_free()
	for station in get_tree().get_nodes_in_group("relay_stations"):
		if powered.has(token(station.station_id)):
			station.power_on()
			open_gate(station.target, true)
	respawn_now()
	player.enabled = not won
	loading = false
	refresh_hud()
	return true
func respawn() -> void:
	message = "The canal returned you to the quay. Your collected energy is safe."
	call_deferred("respawn_now")
func respawn_now() -> void:
	var spawn := get_tree().get_first_node_in_group("relay_spawns") as Node3D
	if spawn:
		player.global_position = spawn.global_position + Vector3.UP * 0.05
		player.velocity = Vector3.ZERO
		player.rotation.y = PI
		player.camera.rotation = Vector3.ZERO
func collect(cell: Area3D) -> void:
	var id := token(cell.cell_id)
	if claimed.has(id) or loading or won:
		return
	claimed[id] = true
	energy += cell.value
	cell.queue_free()
	message = "Energy cell collected. Find the relay station."
	save_progress()
func use_nearest_station() -> void:
	var nearest: Node3D = null
	var distance := 2.6
	for station in get_tree().get_nodes_in_group("relay_stations"):
		if station.active:
			continue
		var aim: Vector3 = station.global_position + Vector3.UP * 1.0
		var delta: Vector3 = aim - player.camera.global_position
		if delta.length() < distance and -player.camera.global_basis.z.dot(delta.normalized()) > 0.5:
			var query := PhysicsRayQueryParameters3D.create(player.camera.global_position, aim)
			query.exclude = [player.get_rid()]
			if get_world_3d().direct_space_state.intersect_ray(query).is_empty():
				nearest = station
				distance = delta.length()
	if nearest:
		activate(nearest)
	else:
		message = "Face a nearby relay station and press F."
func activate(station: Node3D) -> bool:
	if station.active or won:
		return false
	if energy < station.cost:
		message = "This relay needs %d energy. You have %d." % [station.cost, energy]
		return false
	energy -= station.cost
	powered[token(station.station_id)] = true
	station.power_on()
	open_gate(station.target)
	message = "Relay powered. The gate is open. Follow the canal to the exit."
	save_progress()
	return true
func open_gate(target: String, immediate := false) -> void:
	for door in get_tree().get_nodes_in_group("relay_doors"):
		var props: Dictionary = door.get_meta("properties", {})
		if str(props.get("targetname", "")) == target:
			door.open_gate(immediate)
func try_exit(required: String, next_level: String) -> bool:
	if won or not powered.has(token(required)):
		message = "Power this chapter's relay before leaving."
		return false
	call_deferred("advance", next_level)
	return true
func advance(next_level: String) -> void:
	if next_level.is_empty():
		won = true
		player.enabled = false
		Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
		message = "The canal beacon is lit. Campaign complete! Press R to play again."
	else:
		if not load_level(next_level):
			push_error("Unknown campaign chapter: " + next_level)
			return
		message = "New quay, new relay. Collect its energy cells."
	save_progress()
func refresh_hud() -> void:
	if hud:
		var index := CHAPTERS.find(level_name)
		hud.text = "CANAL RELAY  /  %d of 3  /  %s\nEnergy: %d    Time: %.1fs\n%s\nWASD move  ·  Mouse look  ·  Space jump  ·  F use  ·  Esc release  ·  R restart" % [index + 1, TITLES[index], energy, elapsed, message]
func save_progress() -> void:
	var file := FileAccess.open(SAVE, FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify({"version":1,"level":level_name,"energy":energy,"claimed":claimed,"powered":powered,"won":won,"elapsed":elapsed}))
func read_save() -> void:
	if not FileAccess.file_exists(SAVE):
		return
	var data: Variant = JSON.parse_string(FileAccess.get_file_as_string(SAVE))
	if not data is Dictionary or data.get("version") != 1 or data.get("level") not in CHAPTERS:
		return
	level_name = data.level
	energy = maxi(0, int(data.get("energy", 0)))
	claimed = data.get("claimed", {}) if data.get("claimed") is Dictionary else {}
	powered = data.get("powered", {}) if data.get("powered") is Dictionary else {}
	won = bool(data.get("won", false))
	elapsed = maxf(0, float(data.get("elapsed", 0)))
