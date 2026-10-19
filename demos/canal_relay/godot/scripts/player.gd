extends CharacterBody3D
var camera: Camera3D
var enabled := true
func _ready() -> void:
	name = "Player"
	add_to_group("relay_player")
	var shape := CapsuleShape3D.new()
	shape.radius = 0.25
	shape.height = 1.7
	var collider := CollisionShape3D.new()
	collider.shape = shape
	collider.position.y = 0.85
	add_child(collider)
	camera = Camera3D.new()
	camera.position.y = 1.55
	camera.current = true
	add_child(camera)
	floor_snap_length = 0.3
func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED and enabled:
		rotation.y -= event.relative.x * 0.002
		camera.rotation.x = clampf(camera.rotation.x - event.relative.y * 0.002, -1.5, 1.5)
func _physics_process(dt: float) -> void:
	if not enabled:
		return
	if not is_on_floor():
		velocity.y -= 18.0 * dt
	if Input.is_action_just_pressed("jump") and is_on_floor():
		velocity.y = 6.0
	var axes := Input.get_vector("left", "right", "forward", "back")
	var direction := transform.basis * Vector3(axes.x, 0, axes.y)
	velocity.x = direction.x * 5.0
	velocity.z = direction.z * 5.0
	move_and_slide()
