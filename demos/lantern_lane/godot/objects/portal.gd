extends Area3D
## A portal. Walk into it and come out of the one it points at.
##
## heron_brush resolves the Name/Target pair into a NodePath while it builds the scene,
## so nothing is searched for by name here. No Target means exit only.

## How far in front of the far portal you come out, in metres. Enough to clear
## its own volume, or you would be sent straight back.
@export var exit_offset: float = 1.4
## How long the far portal ignores whoever it just received, in seconds.
@export var settle_time: float = 0.6

var destination: Node3D = null

var _cooldown: Dictionary = {}
var _time := 0.0


func _ready() -> void:
	body_entered.connect(_on_body_entered)

	var props: Dictionary = get_meta("properties", {})
	if props.has("color"):
		_tint(props["color"])

	var paths: Array = get_meta("target_paths", [])
	if paths.is_empty():
		return
	destination = get_node_or_null(paths[0])
	if destination == null:
		push_warning("%s points at %s, which is not in the scene" % [name, paths[0]])


func _process(delta: float) -> void:
	_time += delta
	$ring.rotate_y(delta * 0.8)

	# A cooldown that runs down on its own cannot get stuck, which a plain flag
	# can when the far portal never sees the body arrive.
	for body in _cooldown.keys():
		_cooldown[body] -= delta
		if _cooldown[body] <= 0.0:
			_cooldown.erase(body)


func _on_body_entered(body: Node3D) -> void:
	if destination == null or _cooldown.has(body) or not body is CharacterBody3D:
		return

	# Set the far portal's cooldown before moving anyone, so the arrival that
	# follows on the next physics step is already being ignored.
	destination.receive(body)

	var forward: Vector3 = -destination.global_transform.basis.z
	body.global_position = destination.global_position + forward * exit_offset

	# Turn whoever came through to face the way the far portal faces, and carry
	# their momentum round with them.
	var turn: float = destination.global_rotation.y - global_rotation.y
	if body.velocity.length_squared() > 0.0:
		body.velocity = body.velocity.rotated(Vector3.UP, turn)
	_face(body, destination.global_rotation.y)


## Called by the portal that is sending someone here.
func receive(body: Node3D) -> void:
	_cooldown[body] = settle_time


## The kit's player drives its own yaw from `rotation_target`, so setting
## `rotation` alone would be undone by the next mouse movement.
func _face(body: Node3D, yaw: float) -> void:
	body.rotation.y = yaw
	if "rotation_target" in body:
		body.rotation_target.y = yaw


## Recolours both parts while leaving each its own brightness, so the ring keeps
## reading as a ring against the sheet inside it.
func _tint(raw: Variant) -> void:
	var colour: Color = raw if raw is Color else Color(1, 1, 1)
	for part in [$ring, $sheet]:
		var material := part.get_surface_override_material(0) as StandardMaterial3D
		if material == null:
			continue
		material = material.duplicate()
		material.albedo_color = Color(colour.r, colour.g, colour.b, material.albedo_color.a)
		material.emission = colour
		part.set_surface_override_material(0, material)
