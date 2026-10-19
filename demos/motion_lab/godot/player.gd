extends CharacterBody3D
## Ordinary CharacterBody movement. Godot supplies moving-platform velocity.
var input_enabled := true
var desired_move := Vector3.ZERO
var jump_requested := false
var _platform: Node3D
var _platform_local := Vector3.ZERO
var camera: Camera3D
const SPEED := 5.0
const GRAVITY := 22.0
const JUMP_SPEED := 7.0
func _ready() -> void:
	collision_layer = 1
	collision_mask = 1
	platform_floor_layers = 1
	floor_snap_length = 0.3
	platform_on_leave = CharacterBody3D.PLATFORM_ON_LEAVE_ADD_UPWARD_VELOCITY
	process_physics_priority = 10
	var shape := CollisionShape3D.new()
	var capsule := CapsuleShape3D.new()
	capsule.radius = 0.3
	capsule.height = 1.7
	shape.shape = capsule
	shape.position.y = 0.85
	add_child(shape)
	camera = Camera3D.new()
	camera.position.y = 1.5
	camera.fov = 78
	add_child(camera)
	camera.current = true
func _unhandled_input(event: InputEvent) -> void:
	if not input_enabled:
		return
	if event is InputEventMouseMotion and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		rotate_y(-event.relative.x * 0.002)
		camera.rotation.x = clampf(camera.rotation.x - event.relative.y * 0.002, -1.45, 1.45)
	if event is InputEventKey and event.pressed and not event.echo and event.physical_keycode == KEY_SPACE:
		jump_requested = true
func _physics_process(delta: float) -> void:
	var movement := desired_move
	if input_enabled:
		movement = Vector3(float(Input.is_physical_key_pressed(KEY_D)) - float(Input.is_physical_key_pressed(KEY_A)), 0, float(Input.is_physical_key_pressed(KEY_S)) - float(Input.is_physical_key_pressed(KEY_W)))
		movement = basis * movement.normalized()
	velocity.x = movement.x * SPEED
	velocity.z = movement.z * SPEED
	if is_on_floor():
		velocity.y = 0
	else:
		velocity.y -= GRAVITY * delta
	if jump_requested and is_on_floor():
		velocity.y = JUMP_SPEED
	jump_requested = false
	move_and_slide()
	# Godot follows angular velocity by small linear steps. On an idle rider
	# these accumulate radial drift. Correct only that small tangent error,
	# through collision detection, while retaining normal walking and jumping.
	var floor_body: Node3D = null
	if is_on_floor():
		for i in get_slide_collision_count():
			var hit := get_slide_collision(i)
			if hit.get_normal().dot(Vector3.UP) > 0.7:
				floor_body = hit.get_collider() as Node3D
				break
	if is_instance_valid(_platform) and floor_body == _platform and movement.length_squared() < 0.0001 and get_platform_angular_velocity().length_squared() > 0.0001:
		var correction: Vector3 = _platform.to_global(_platform_local) - global_position
		correction.y = 0
		if correction.length() < 0.1:
			move_and_collide(correction)
	_platform = floor_body
	if is_instance_valid(_platform):
		_platform_local = _platform.to_local(global_position)
