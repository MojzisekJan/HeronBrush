extends Camera3D
class_name Spectator
## A camera that flies through the level, for looking at what the map turned
## into: the top of a lintel, the back of a crate, the faces you never stand in
## front of and which end up carrying a texture nobody sees.
##
## The readout gives the point under the crosshair in map units as well as
## Godot ones, so what looks wrong here can be found in the level editor.

## Map units per Godot unit, the same number heron_brush built the scene with. It is
## only used to turn the readout back into something the level editor understands.
@export var map_units: float = 32.0

const BASE_SPEED := 6.0
const BOOST := 4.0
const CREEP := 0.25
const MOUSE_SENSITIVITY := 700.0
const REACH := 200.0
## How often the crosshair asks what it is pointing at. Every frame would be
## wasteful and the answer does not change fast enough to notice.
const PROBE_EVERY := 0.1

## Godot's debug views. Wireframe answers whether the join worked; overdraw
## paints each pixel by how often it was drawn, so buried faces glow.
const MODES := [
	Viewport.DEBUG_DRAW_DISABLED,
	Viewport.DEBUG_DRAW_WIREFRAME,
	Viewport.DEBUG_DRAW_OVERDRAW,
	Viewport.DEBUG_DRAW_UNSHADED,
]
const MODE_NAMES := ["normal", "wireframe", "overdraw", "unshaded"]

var speed := BASE_SPEED
var yaw := 0.0
var pitch := 0.0
var mode := 0
var since_probe := 0.0
var readout: RichTextLabel
var probe := {}


func _ready() -> void:
	# Without this the wireframe view renders nothing: the meshes go up without
	# the extra index buffer, and Godot only builds it when told to beforehand.
	RenderingServer.set_debug_generate_wireframes(true)
	yaw = global_rotation.y
	pitch = global_rotation.x
	current = true
	build_readout()


func _exit_tree() -> void:
	get_viewport().debug_draw = Viewport.DEBUG_DRAW_DISABLED


func build_readout() -> void:
	var layer := CanvasLayer.new()
	layer.layer = 10
	add_child(layer)
	var panel := PanelContainer.new()
	panel.position = Vector2(12, 12)
	panel.custom_minimum_size = Vector2(460, 0)
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0, 0, 0, 0.55)
	style.content_margin_left = 10
	style.content_margin_right = 10
	style.content_margin_top = 8
	style.content_margin_bottom = 8
	panel.add_theme_stylebox_override("panel", style)
	layer.add_child(panel)
	readout = RichTextLabel.new()
	readout.bbcode_enabled = true
	readout.fit_content = true
	readout.custom_minimum_size = Vector2(440, 0)
	readout.add_theme_font_size_override("normal_font_size", 14)
	panel.add_child(readout)

	# The HUD is off while flying, and without something in the middle of the
	# screen the readout describes a spot you cannot see.
	var cross := Label.new()
	cross.text = "+"
	cross.set_anchors_preset(Control.PRESET_FULL_RECT)
	cross.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	cross.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	cross.add_theme_font_size_override("font_size", 22)
	cross.mouse_filter = Control.MOUSE_FILTER_IGNORE
	layer.add_child(cross)


func _input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		yaw -= event.relative.x / MOUSE_SENSITIVITY
		pitch = clampf(pitch - event.relative.y / MOUSE_SENSITIVITY,
			deg_to_rad(-89.0), deg_to_rad(89.0))
	# Reading a texture wants centimetres a second, crossing an arena wants ten
	# metres.
	if event is InputEventMouseButton and event.pressed:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			speed = minf(speed * 1.25, 200.0)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			speed = maxf(speed / 1.25, 0.25)


func _process(delta: float) -> void:
	global_rotation = Vector3(pitch, yaw, 0.0)

	if Input.is_action_just_pressed("view_mode"):
		mode = (mode + 1) % MODES.size()
		get_viewport().debug_draw = MODES[mode]

	var move := Input.get_vector("move_left", "move_right", "move_forward", "move_back")
	var wish := global_basis * Vector3(move.x, 0.0, move.y)
	if Input.is_action_pressed("jump"):
		wish += Vector3.UP
	if Input.is_action_pressed("crouch"):
		wish += Vector3.DOWN
	var pace := speed
	if Input.is_action_pressed("sprint"):
		pace *= BOOST
	elif Input.is_action_pressed("use"):
		# Creeping up on a face, for reading a texture seam.
		pace *= CREEP
	global_position += wish.normalized() * pace * delta

	since_probe -= delta
	if since_probe <= 0.0:
		since_probe = PROBE_EVERY
		probe = what_is_in_front()
	show_readout()


## In map axes and map units, which is what the level editor shows. heron_brush swizzles
## Quake's z-up into Godot's y-up on the way in; this is that undone.
func to_map(point: Vector3) -> Vector3:
	return Vector3(point.x, point.z, point.y) * map_units


func show_readout() -> void:
	var here := to_map(global_position)
	var lines := PackedStringArray()
	lines.append("[b]FLY[/b]   %.1f m/s   [F3] back to the player   [F4] %s" % [
		speed, MODE_NAMES[mode]])
	lines.append("you   godot %.1f %.1f %.1f   map %d %d %d" % [
		global_position.x, global_position.y, global_position.z, here.x, here.y, here.z])
	if probe.is_empty():
		lines.append("[color=#888]looking at nothing within %d m[/color]" % int(REACH))
	else:
		var spot: Vector3 = to_map(probe["point"])
		lines.append("looking at   [b]%s[/b] / [b]%s[/b]   %.1f m away" % [
			probe["node"], probe["texture"], probe["distance"]])
		lines.append("map %d %d %d   surface %s   %d triangles on it" % [
			spot.x, spot.y, spot.z, probe["surface"], probe["triangles"]])
	readout.text = "\n".join(lines)


## What the crosshair is on: the node, and which surface of its mesh, which is
## the same thing as which texture. Areas are left out; a trigger is not
## something you can look at.
func what_is_in_front() -> Dictionary:
	var to: Vector3 = global_position - global_basis.z * REACH
	var query := PhysicsRayQueryParameters3D.create(global_position, to)
	query.collide_with_areas = false
	var hit: Dictionary = get_world_3d().direct_space_state.intersect_ray(query)
	if hit.is_empty():
		return {}

	var found := {
		"node": String(hit["collider"].name),
		"point": hit["position"],
		"distance": global_position.distance_to(hit["position"]),
		"texture": "?",
		"surface": "?",
		"triangles": 0,
	}
	var mesh := mesh_of(hit["collider"])
	if mesh == null or mesh.mesh == null:
		return found

	var local: Vector3 = mesh.to_local(hit["position"])
	for surface in mesh.mesh.get_surface_count():
		var arrays: Array = mesh.mesh.surface_get_arrays(surface)
		var points: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var index: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
		if not sits_on_surface(local, points, index):
			continue
		found["texture"] = texture_name(mesh, surface)
		found["surface"] = "%d of %d" % [surface + 1, mesh.mesh.get_surface_count()]
		found["triangles"] = index.size() / 3
		break
	return found


func mesh_of(body: Node) -> MeshInstance3D:
	# heron_brush calls it "mesh"; a scene built by hand in Godot may not.
	var named := body.get_node_or_null("mesh")
	if named is MeshInstance3D:
		return named
	for child in body.get_children():
		if child is MeshInstance3D:
			return child
	return null


func sits_on_surface(point: Vector3, points: PackedVector3Array, index: PackedInt32Array) -> bool:
	for i in range(0, index.size(), 3):
		if in_triangle(point, points[index[i]], points[index[i + 1]], points[index[i + 2]]):
			return true
	return false


## Whether the point is on the triangle. The three corner-to-point triangles
## add up to the whole one exactly when the point is inside it.
func in_triangle(p: Vector3, a: Vector3, b: Vector3, c: Vector3) -> bool:
	var cross := (b - a).cross(c - a)
	var twice_area := cross.length()
	if twice_area < 1e-9:
		return false
	if absf((p - a).dot(cross / twice_area)) > 0.02:
		return false
	var parts: float = (b - p).cross(c - p).length() \
		+ (c - p).cross(a - p).length() \
		+ (a - p).cross(b - p).length()
	return parts <= twice_area * 1.01


func texture_name(mesh: MeshInstance3D, surface: int) -> String:
	var material: Material = mesh.get_surface_override_material(surface)
	if material == null:
		material = mesh.mesh.surface_get_material(surface)
	if material == null:
		return "no material"
	if material is StandardMaterial3D and material.albedo_texture != null:
		return material.albedo_texture.resource_path.get_file().get_basename()
	if material is ShaderMaterial:
		if material.resource_path != "":
			return material.resource_path.get_file().get_basename() + " (shader)"
		return "shader"
	if material.resource_path != "":
		return material.resource_path.get_file().get_basename()
	return "untextured"
