extends Node3D
var player: CharacterBody3D
var map: Node3D
var status: Label
var testing := false
var elapsed := 0.0
const STATIONS := [Vector3(-14, 1.4, -6), Vector3(6, 0.8, -6), Vector3(-6, 0.8, 7), Vector3(8, 0.2, 3)]
func _ready() -> void:
	map = load("res://maps/motion_lab.tscn").instantiate()
	add_child(map)
	var environment := WorldEnvironment.new()
	var settings := Environment.new()
	settings.background_mode = Environment.BG_COLOR
	settings.background_color = Color("172335")
	settings.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	settings.ambient_light_color = Color("b6cee2")
	settings.ambient_light_energy = 0.35
	settings.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	environment.environment = settings
	add_child(environment)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-55, -35, 0)
	sun.light_energy = 0.85
	sun.shadow_enabled = true
	add_child(sun)
	player = CharacterBody3D.new()
	player.set_script(load("res://player.gd"))
	player.input_enabled = not testing
	player.position = Vector3(0, 0.2, 11)
	add_child(player)
	for item in [["01 / L-PATH",Vector3(-10,3,-9),Color("28ceb5")],["02 / LIFT",Vector3(6,8,-10),Color("ffa141")],["03 / ROTATION",Vector3(-7,3,11),Color("5b97ff")],["04 / DOOR",Vector3(8,5,6.2),Color("eb6d9e")]]:
		var label := Label3D.new()
		label.text = item[0]
		label.position = item[1]
		label.modulate = item[2]
		label.font_size = 64
		label.pixel_size = 0.009
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		add_child(label)
	var hud := CanvasLayer.new()
	add_child(hud)
	var panel := PanelContainer.new()
	panel.position = Vector2(24,24)
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.035,0.055,0.085,0.92)
	style.content_margin_left = 18
	style.content_margin_right = 18
	style.content_margin_top = 14
	style.content_margin_bottom = 14
	style.corner_radius_top_left = 10
	style.corner_radius_bottom_right = 10
	panel.add_theme_stylebox_override("panel",style)
	hud.add_child(panel)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation",8)
	panel.add_child(column)
	var heading := Label.new()
	heading.text = "HERON BRUSH   /   MOTION LAB"
	heading.add_theme_font_size_override("font_size",22)
	column.add_child(heading)
	var help := Label.new()
	help.text = "WASD + mouse · Space: jump · Esc: release mouse\n1–4: visit station · R: restart station\nWalk onto a colored pad to activate its target."
	column.add_child(help)
	status = Label.new()
	status.modulate = Color("28ceb5")
	column.add_child(status)
	if not testing:
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
func animated(name: String) -> Node3D:
	for node in map.find_children("*", "AnimatableBody3D", true, false):
		if node.get_meta("properties",{}).get("targetname","") == name:
			return node
	return null
func _unhandled_input(event: InputEvent) -> void:
	if testing:
		return
	if event is InputEventKey and event.pressed and not event.echo:
		if event.physical_keycode == KEY_ESCAPE:
			Input.mouse_mode = Input.MOUSE_MODE_VISIBLE if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED else Input.MOUSE_MODE_CAPTURED
		elif event.physical_keycode >= KEY_1 and event.physical_keycode <= KEY_4:
			visit(event.physical_keycode - KEY_1)
		elif event.physical_keycode == KEY_R:
			visit(current_station)
var current_station := 0
func visit(index: int) -> void:
	current_station = index
	var body := animated(["shuttle","lift","rotor","door"][index])
	body.get_node("HeronAnimation").stop()
	player.velocity = Vector3.ZERO
	player.position = STATIONS[index]
	player.rotation = Vector3.ZERO
	player.camera.rotation = Vector3.ZERO
	if index < 3:
		body.get_node("HeronAnimation").play()
func _process(delta: float) -> void:
	elapsed += delta
	if not testing and player.position.y < -8:
		visit(current_station)
	status.text = "On floor: %s   |   Platform speed: %.2f m/s" % ["yes" if player.is_on_floor() else "no",player.get_platform_velocity().length()]
