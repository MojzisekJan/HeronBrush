extends CharacterBody3D

@export_subgroup("Properties")
@export var movement_speed = 5
@export var sprint_multiplier = 1.7
@export_range(0, 100) var number_of_jumps: int = 2
@export var jump_strength = 8

@export_subgroup("Steps")
## The tallest step climbed without jumping, in metres. Zero turns it off.
@export var max_step_height = 0.5
## How fast the camera catches up after a step. Without it the view jumps.
@export var step_camera_lerp = 8.0
## How many heights to try before giving up on a step, tallest first. One try
## alone misses a step with a beam just above it.
@export var step_tries = 2

@export_subgroup("View")
## How far the view leans when moving sideways, in degrees. Quake's cl_rollangle.
@export var strafe_roll_degrees = 2.0
@export var strafe_roll_speed = 5.0
## How far the weapon may swing behind the movement, in metres. The kit never
## capped this, which throws the weapon off screen at strafe jumping speed.
@export var weapon_sway_limit = 0.1

@export_subgroup("Air control")
@export var ground_accelerate = 14.0
@export var friction = 8.0
## How hard you can push yourself while off the ground. Well under the ground
## figure, or a jump becomes a walk.
@export var air_accelerate = 2.2
## Ceiling on how fast one push carries you along the way you look. Tight is
## Quake 1, zero is Quake 3. Strafe jumping works either way.
@export var air_speed_cap = 0.0
@export var wall_jump_strength = 7.0
@export var wall_jump_push = 5.0
@export var wall_jump_cooldown = 0.25

@export_subgroup("Crouching")
## Capsule height while ducked, against 1.2 standing. Godot will not take one
## below twice the radius, which is 0.72 here.
@export var crouch_height = 0.72
## Top speed while ducked, and the speed a slide may be steered with.
@export var crouch_speed = 2.5
## How fast you duck and rise again, in capsule heights per second.
@export var crouch_lerp = 12.0
## How fast you have to be going for Crouch to start a slide instead of a duck.
@export var slide_min_speed = 6.0
## The speed a slide starts at. Coming in faster keeps whatever you had, so
## slide, jump, slide cannot compound.
@export var slide_speed = 8.5
## Friction while sliding, against `friction` on foot. What makes a slide carry.
@export var slide_friction = 1.5
@export var slide_time = 0.9

@export_subgroup("Climbing")
@export var climb_speed = 3.0
## Sideways movement on a ladder, against climbing speed. Well under one.
@export var climb_sideways = 0.4
## How hard Jump throws you off a ladder, away from where you are looking.
@export var ladder_push = 5.0
## Seconds before the same ladder can be grabbed again. Without it you are back
## inside the volume the frame after jumping off.
@export var ladder_cooldown = 0.35

@export_subgroup("Swimming")
## How fast you move through water, against movement_speed on land.
@export var swim_speed = 3.0
## How fast Jump carries you up and Sprint carries you down.
@export var swim_vertical_speed = 3.5
@export var swim_sink_speed = 0.8
## How quickly the water takes hold of you. Also what stops a long fall dead.
@export var swim_grip = 14.0

@export_subgroup("Weapons")
@export var weapons: Array[Weapon] = []

var weapon: Weapon
var weapon_index := 0

var mouse_sensitivity = 700
var gamepad_sensitivity := 0.075
## Set from the pause menu. Flips which way looking up is.
var invert_look := false

var mouse_captured := true

var movement_velocity: Vector3
var rotation_target: Vector3

var input_mouse: Vector2

var health: int = 100
var gravity := 0.0

## How many water volumes we are inside; they can overlap.
var water_volumes := 0
var water_sink := 0.0
var water_rise := 0.0
var water_speed := 0.0
## Where the top of the water we are in sits, in world space.
var water_surface := 0.0
@onready var underwater: MeshInstance3D = $Head/Camera/Underwater
var is_swimming: bool:
	get:
		return water_volumes > 0

## How many ladders we are inside, and how long until one can be grabbed again.
var ladders := 0
var ladder_timer := 0.0
var ladder_speed := 0.0
## What the movement keys asked for this frame, before anything is done with it.
var move_input := Vector2.ZERO
var is_climbing: bool:
	get:
		return ladders > 0 and ladder_timer <= 0.0

var previously_floored := false

var wall_jump_timer := 0.0

## Ducked, either because Crouch is held or because there is no room to rise.
var is_crouching := false
## 0 stood up, 1 fully ducked. The capsule and the head both follow it.
var crouch_amount := 0.0
## Seconds of slide left. Zero means not sliding.
var slide_timer := 0.0
var is_sliding: bool:
	get:
		return slide_timer > 0.0
@onready var collider: CollisionShape3D = $Collider
var stand_height := 1.0
## Where the bottom of the capsule sits. Ducking shrinks it from the top.
var collider_bottom := 0.0

## How far the head still has to catch up after the body was lifted onto a step.
var head_step_offset := 0.0
@onready var head_rest_y: float = $Head.position.y
## Set on the frame a jump is made, so the landing frame does not scrub the
## speed off. Without it a bunny hop loses a fifth of its speed each touchdown.
var jumped_this_frame := false

var jumps_remaining: int

var container_offset = Vector3(1.2, -1.1, -2.75)

var tween: Tween

signal health_updated

@onready var camera = $Head/Camera
@onready var raycast = $Head/Camera/RayCast
@onready var muzzle = $Head/Camera/SubViewportContainer/SubViewport/CameraItem/Muzzle
@onready var container = $Head/Camera/SubViewportContainer/SubViewport/CameraItem/Container
@onready var sound_footsteps = $SoundFootsteps
@onready var blaster_cooldown = $Cooldown

@export var crosshair: TextureRect

# Functions

func _ready():
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED

	# The shape is written to every frame while ducking, and a sub-resource is
	# shared by every instance of the scene, so it gets a copy of its own first.
	collider.shape = collider.shape.duplicate()
	# Same for the underwater quad, which is resized to fit the view, and for
	# its material, which carries the height of whichever pool this player is in.
	underwater.mesh = underwater.mesh.duplicate()
	underwater.set_surface_override_material(0,
		underwater.get_surface_override_material(0).duplicate())
	stand_height = collider.shape.height
	collider_bottom = collider.position.y - stand_height * 0.5

	weapon = weapons[weapon_index] # Weapon must never be nil
	initiate_change_weapon(weapon_index)

func _process(delta):
	# Handle functions
	jumped_this_frame = false
	handle_controls(delta)
	handle_crouch(delta)
	handle_gravity(delta)

	# Movement

	movement_velocity = transform.basis * movement_velocity # Move forward

	if is_climbing:
		handle_climb(delta)
	elif is_swimming:
		# Water is not Quake. Keep the simple pull towards where you point.
		velocity.x = lerp(velocity.x, movement_velocity.x, delta * 10)
		velocity.z = lerp(velocity.z, movement_velocity.z, delta * 10)
	else:
		handle_acceleration(delta)

	# Climbing drives the whole velocity, up included, so gravity would undo it.
	if not is_climbing:
		velocity.y = - gravity

	var applied_velocity := velocity

	var before := global_position
	var carried := velocity
	move_and_slide()

	if max_step_height > 0.0 and gravity >= 0.0:
		var got := Vector2(global_position.x - before.x, global_position.z - before.z).length()
		var wanted: float = Vector2(carried.x, carried.z).length() * delta
		# Trying to move and not getting there is the only reliable sign of a step.
		# `is_on_wall` misses them: a capsule meets a step on its rounded bottom and
		# that contact comes back as floor.
		if wanted > 0.001 and got < wanted * 0.9:
			try_stair_move(before, carried, delta)

	wall_jump_timer = max(0.0, wall_jump_timer - delta)
	ladder_timer = max(0.0, ladder_timer - delta)

	# Rotation
	var sway: Vector3 = basis.inverse() * applied_velocity / 30.0
	if sway.length() > weapon_sway_limit:
		sway = sway.normalized() * weapon_sway_limit
	container.position = lerp(container.position, container_offset - sway, delta * 10)

	handle_view_roll(delta)

	# The body is lifted onto a step in one go, so the head is held back by the
	# same amount and let catch up over a few frames. Otherwise the view jumps.
	head_step_offset = lerp(head_step_offset, 0.0, min(delta * step_camera_lerp, 1.0))
	# Ducking takes the top off the capsule, and the head goes down by exactly
	# as much, so the view sits where the eyes would be either way.
	$Head.position.y = head_rest_y - (stand_height - collider.shape.height) - head_step_offset

	show_underwater()

	# Movement sound

	sound_footsteps.stream_paused = true

	if is_on_floor():
		if abs(velocity.x) > 1 or abs(velocity.z) > 1:
			sound_footsteps.stream_paused = false

	# Landing after jump or falling

	camera.position.y = lerp(camera.position.y, 0.0, delta * 5)

	if is_on_floor() and gravity > 1 and !previously_floored: # Landed
		Audio.play("sounds/land.ogg")
		camera.position.y = -0.1

	previously_floored = is_on_floor()

	# Falling/respawning

	if position.y < -10:
		# Falling out of the world is a death like any other, so it goes back to
		# the last checkpoint rather than throwing the whole run away.
		back_to_the_start()

## Puts the underwater view in front of the eye, or takes it away. Shown for as
## long as the body is in water; the shader throws away everything above the
## surface, so a half-submerged head gives a line across the screen.
func show_underwater():
	if underwater == null:
		return
	# Being in the water is not the same as looking out of it, so the eye decides.
	# The shader guards this too; doing it here saves drawing the quad for nothing.
	underwater.visible = is_swimming and camera.global_position.y < water_surface
	if not underwater.visible:
		return
	var material: ShaderMaterial = underwater.get_surface_override_material(0)
	if material == null:
		return
	material.set_shader_parameter("water_level", water_surface)

	# Sized to cover the view. Field of view is a setting now and the window can
	# be resized, so a quad cut to fit once would stop fitting.
	var quad: QuadMesh = underwater.mesh
	var reach: float = absf(underwater.position.z)
	var tall: float = 2.0 * reach * tan(deg_to_rad(camera.fov) * 0.5)
	var window: Vector2 = get_viewport().get_visible_rect().size
	var wide: float = tall * (window.x / max(window.y, 1.0))
	# Well over, so no rounding or aspect change can bring the edge of the quad
	# into the picture.
	quad.size = Vector2(wide, tall) * 1.2


## Whoever is running the level gets first refusal, so a checkpoint can be
## honoured. Without one, fall back to starting the level again.
func back_to_the_start():
	var level := get_parent()
	if level != null and level.has_method("respawn_player"):
		level.respawn_player(self)
	elif get_tree().current_scene != null:
		get_tree().reload_current_scene()

# Mouse movement

func _input(event):
	if event is InputEventMouseMotion and mouse_captured:
		input_mouse = event.relative / mouse_sensitivity
		handle_rotation(event.relative.x, event.relative.y, false)

func handle_controls(delta):
	# Mouse capture
	if Input.is_action_just_pressed("mouse_capture"):
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
		mouse_captured = true

	# Movement
	var input := Input.get_vector("move_left", "move_right", "move_forward", "move_back")
	move_input = input

	# Sprint is forwards and on the ground only. In the air it would raise the
	# ceiling on what a push can reach and hand out speed from nowhere. Momentum
	# still carries a sprint through a jump.
	var speed = movement_speed
	if is_swimming:
		speed = water_speed
	elif is_crouching or is_sliding:
		# Sprint is deliberately out of reach here: ducking is the slow way through a
		# gap, and a slide spends speed rather than making it.
		speed = crouch_speed
	elif is_on_floor() and Input.is_action_pressed("sprint") and input.y < 0:
		speed *= sprint_multiplier

	movement_velocity = Vector3(input.x, 0, input.y).normalized() * speed

	# Handle Controller Rotation
	var rotation_input := Input.get_vector("camera_right", "camera_left", "camera_down", "camera_up")
	if rotation_input:
		handle_rotation(rotation_input.x, rotation_input.y, true, delta)

	# Shooting

	action_shoot()

	# Jumping

	if is_climbing and Input.is_action_just_pressed("jump"):
		action_leave_ladder()
	elif is_on_floor():
		# Auto hop. Without it a player holding the key gets one jump and then runs,
		# where friction takes back everything the turn in the air earned.
		if Input.is_action_pressed("jump"):
			jumps_remaining = number_of_jumps   # a hop off the ground is free
			action_jump()
	elif Input.is_action_just_pressed("jump"):
		# A wall jump costs a jump like any other, so a corridor can be climbed
		# only as far as the jumps in hand allow.
		if is_on_wall() and wall_jump_timer <= 0.0 and jumps_remaining:
			action_wall_jump()
		elif jumps_remaining:
			action_jump()

	# Weapon switching

	action_weapon_toggle()

# Crouching and sliding

## Ducking, and the slide that comes out of ducking at speed. A slide keeps the
## speed you came in with; jumping out of one keeps the lot.
func handle_crouch(delta):
	var want: bool = Input.is_action_pressed("crouch") and not is_swimming
	var flat := Vector2(velocity.x, velocity.z)

	if slide_timer > 0.0:
		slide_timer -= delta
		# A slide is a thing that happens on the floor. Leaving it, letting go,
		# or dropping to a crawl all end it; the speed is kept either way.
		if not want or not is_on_floor() or flat.length() < crouch_speed:
			slide_timer = 0.0
	elif want and not is_crouching and is_on_floor() and flat.length() >= slide_min_speed:
		start_slide(flat)

	# Standing up is not always allowed. Under a low ceiling the duck holds
	# until there is room, or the body would be pushed into the geometry.
	if want:
		is_crouching = true
	elif is_crouching and can_stand():
		is_crouching = false

	crouch_amount = move_toward(crouch_amount, 1.0 if is_crouching else 0.0, delta * crouch_lerp)
	var height: float = lerp(stand_height, crouch_height, crouch_amount)
	collider.shape.height = height
	collider.position.y = collider_bottom + height * 0.5


## Whether there is room overhead to go back to full height.
func can_stand() -> bool:
	var missing: float = stand_height - collider.shape.height
	if missing <= 0.001:
		return true
	# The capsule only grows upwards, so testing the ducked one moving up by
	# what it is short of covers exactly the space the taller one would want.
	return not test_move(global_transform, Vector3.UP * missing)


func start_slide(flat: Vector2):
	slide_timer = slide_time
	# Never slower than you came in. Sliding in at 12 keeps 12; sliding in at
	# the minimum lifts you to `slide_speed` and no further.
	if flat.length() < slide_speed:
		var pushed: Vector2 = flat.normalized() * slide_speed
		velocity.x = pushed.x
		velocity.z = pushed.y


# Ladders

## Climbs towards wherever the head is pointing. Nothing is taken from the
## ladder itself: an aimed ladder is one more thing to get wrong.
func handle_climb(delta):
	var wanted: float = ladder_speed if ladder_speed > 0.0 else climb_speed
	var look: Vector3 = -camera.global_basis.z
	var side: Vector3 = camera.global_basis.x
	# move_input.y is negative for forwards, which is what puts the head's own
	# direction under the forward key.
	var wish: Vector3 = look * -move_input.y + side * move_input.x * climb_sideways

	if wish.length() > 0.01:
		velocity = wish.normalized() * wanted
	else:
		# Let go of the keys and you hang where you are rather than sliding back
		# down. A ladder you fall off while thinking is not a ladder.
		velocity = velocity.lerp(Vector3.ZERO, min(delta * 12.0, 1.0))

	gravity = 0.0
	jumps_remaining = number_of_jumps


## Throws you off, away from where you are looking.
func action_leave_ladder():
	Audio.play("sounds/jump_a.ogg, sounds/jump_b.ogg, sounds/jump_c.ogg")
	var away: Vector3 = -camera.global_basis.z
	velocity = Vector3(away.x, 0.0, away.z).normalized() * ladder_push
	gravity = - jump_strength * 0.6
	jumps_remaining = max(0, number_of_jumps - 1)
	jumped_this_frame = true
	# Held off for a moment, or leaving the volume and coming straight back into
	# it would put us on the ladder again before the jump was worth anything.
	ladder_timer = ladder_cooldown


func enter_ladder(speed := 0.0):
	ladders += 1
	ladder_speed = speed
	gravity = 0.0


func exit_ladder():
	ladders = max(0, ladders - 1)
	if ladders == 0:
		ladder_speed = 0.0


## Leans the view into a sideways move. Without it strafing reads as the world
## sliding rather than the player leaning.
func handle_view_roll(delta):
	var sideways: float = velocity.dot(transform.basis.x)
	var lean: float = clampf(sideways / strafe_roll_speed, -1.0, 1.0)
	var target: float = -lean * deg_to_rad(strafe_roll_degrees)
	camera.rotation.z = lerp_angle(camera.rotation.z, target, delta * 8.0)

# Camera rotation

func handle_rotation(xRot: float, yRot: float, isController: bool, delta: float = 0.0):
	var pitch: float = -yRot if invert_look else yRot
	if isController:
		rotation_target -= Vector3(-pitch, -xRot, 0).limit_length(1.0) * gamepad_sensitivity
		rotation_target.x = clamp(rotation_target.x, deg_to_rad(-90), deg_to_rad(90))
		camera.rotation.x = lerp_angle(camera.rotation.x, rotation_target.x, delta * 25)
		rotation.y = lerp_angle(rotation.y, rotation_target.y, delta * 25)
	else:
		rotation_target += (Vector3(-pitch, -xRot, 0) / mouse_sensitivity)
		rotation_target.x = clamp(rotation_target.x, deg_to_rad(-90), deg_to_rad(90))
		camera.rotation.x = rotation_target.x;
		rotation.y = rotation_target.y;

# Handle gravity

func handle_gravity(delta):
	if is_swimming:
		handle_swimming(delta)
		return

	gravity += 20 * delta

	if gravity < 0 and is_on_ceiling():
		gravity = 0

	if gravity > 0 and is_on_floor():
		jumps_remaining = number_of_jumps
		gravity = 0


## Water replaces falling rather than damping it: `gravity` is the one number
## the body uses vertically, so swimming drives it from the controls. Looking
## up or down while swimming forward carries you that way.
func handle_swimming(delta):
	var rise := 0.0

	var input := Input.get_vector("move_left", "move_right", "move_forward", "move_back")
	if not is_zero_approx(input.y):
		# Forward is -1 on that axis, and camera pitch is positive looking up.
		rise += -input.y * sin(camera.rotation.x) * water_speed

	if Input.is_action_pressed("jump"):
		rise += water_rise
	if Input.is_action_pressed("sprint"):
		rise -= water_rise

	if is_zero_approx(rise):
		rise = -water_sink

	gravity = move_toward(gravity, -rise, swim_grip * delta)


## Called by a water volume when this body goes in or comes out. A volume that
## passes nothing gets the player's own defaults.
func enter_water(sink := 0.0, rise := 0.0, speed := 0.0, surface := 0.0):
	water_volumes += 1
	water_sink = sink if sink > 0.0 else swim_sink_speed
	water_rise = rise if rise > 0.0 else swim_vertical_speed
	water_speed = speed if speed > 0.0 else swim_speed
	water_surface = surface
	# A long fall should not carry on to the bottom of the pool.
	gravity = min(gravity, 4.0)
	jumps_remaining = number_of_jumps


func exit_water():
	water_volumes = max(0, water_volumes - 1)

# Steps

## Takes the move again from a step higher, and keeps whichever went further.
##
## `move_and_slide` treats a step as a wall, so the way round is Source's:
## rewind, rise, repeat the same move, drop back down. The move repeated is the
## one already being made, or the stairs hand out distance and the body climbs
## with the keys let go. All of it goes through `move_and_collide`, which leaves
## the body's footing alone.
func try_stair_move(start_pos: Vector3, start_vel: Vector3, delta: float) -> void:
	var plain_pos := global_position
	var plain_gain := _flat_between(start_pos, plain_pos)

	# Several heights, tallest first. Only the tallest misses a step with a beam
	# just above it, where the lift puts the body into the beam.
	var tries: int = max(step_tries, 1)
	for i in range(tries):
		var risen: float = max_step_height * float(tries - i) / float(tries)
		if risen < 0.02:
			break

		global_position = start_pos

		# Straight up, without asking: the body is touching the step, so a swept move
		# reports that contact and travels nothing. Nothing is committed to yet.
		global_position.y += risen

		# Sideways only. Carrying the downward part of the velocity into this
		# drags the body straight back onto the face of the step.
		move_and_collide(Vector3(start_vel.x, 0.0, start_vel.z) * delta)
		var down_hit := move_and_collide(Vector3.DOWN * (risen + 0.05))

		# Two ways of being satisfied, because neither is enough alone. A capsule
		# landing on a step's edge gets a blended normal, and it reads shallower the
		# fatter the capsule: at radius 0.3 floor, at 0.36 wall. Ending higher than we
		# started is the real question.
		var landed: bool = down_hit != null and (
			down_hit.get_normal().y > 0.5
			or global_position.y > start_pos.y + 0.005)
		if landed and _flat_between(start_pos, global_position) > plain_gain + 0.0005:
			# Give back the speed the collision scrubbed off, or every step is climbed
			# from a standstill, and hold the head back by however far the body rose.
			velocity.x = start_vel.x
			velocity.z = start_vel.z
			head_step_offset += max(global_position.y - start_pos.y, 0.0)
			return

	global_position = plain_pos


func _flat_between(a: Vector3, b: Vector3) -> float:
	return Vector2(b.x - a.x, b.z - a.z).length()


# Acceleration, the Quake way

## Ground and air movement, kept apart. In the air each push can only add
## `air_speed_cap` along the way you face, so pointing slightly across your
## path bends it without costing speed. Do that every jump and you get faster.
func handle_acceleration(delta):
	var wish_speed := Vector2(movement_velocity.x, movement_velocity.z).length()
	var wish_dir := Vector3(movement_velocity.x, 0.0, movement_velocity.z)
	if wish_speed > 0.0:
		wish_dir /= wish_speed

	if is_on_floor() and not jumped_this_frame:
		apply_friction(delta)
		if wish_speed > 0.0:
			accelerate(wish_dir, wish_speed, ground_accelerate, delta)
	elif wish_speed > 0.0:
		# A jump made this frame counts as being in the air already, which is
		# what lets speed carry through a touchdown.
		var air_wish := wish_speed
		if air_speed_cap > 0.0:
			air_wish = min(air_wish, air_speed_cap)
		accelerate(wish_dir, air_wish, air_accelerate, delta)


## Adds speed along `dir`, but never past `wish_speed` measured along `dir`.
## Speed already going another way is left alone, which is the whole trick.
func accelerate(dir: Vector3, wish_speed: float, rate: float, delta: float):
	var current := Vector3(velocity.x, 0.0, velocity.z).dot(dir)
	var missing := wish_speed - current
	if missing <= 0.0:
		return
	var push: float = min(rate * wish_speed * delta, missing)
	velocity.x += dir.x * push
	velocity.z += dir.z * push


func apply_friction(delta):
	var speed := Vector2(velocity.x, velocity.z).length()
	if speed < 0.05:
		velocity.x = 0.0
		velocity.z = 0.0
		return
	# The floor of 1.0 stops friction becoming useless at a crawl.
	var rate: float = slide_friction if is_sliding else friction
	var drop: float = max(speed, 1.0) * rate * delta
	# Not named scale: Node3D already has a property by that name.
	var left: float = max(speed - drop, 0.0) / speed
	velocity.x *= left
	velocity.z *= left

# Jumping

func action_jump():
	Audio.play("sounds/jump_a.ogg, sounds/jump_b.ogg, sounds/jump_c.ogg")
	gravity = - jump_strength
	jumps_remaining = max(0, jumps_remaining - 1)
	jumped_this_frame = true


## Kicks off a wall, up and away. Spends a jump, so it chooses where a jump
## takes you rather than handing out extra ones.
func action_wall_jump():
	Audio.play("sounds/jump_c.ogg")
	var away := get_wall_normal()
	gravity = - wall_jump_strength
	velocity.x += away.x * wall_jump_push
	velocity.z += away.z * wall_jump_push
	jumps_remaining = max(0, jumps_remaining - 1)
	wall_jump_timer = wall_jump_cooldown
	jumped_this_frame = true

# Shooting

func action_shoot():
	if Input.is_action_pressed("shoot"):
		if !blaster_cooldown.is_stopped(): return # Cooldown for shooting

		Audio.play(weapon.sound_shoot)

		# Set muzzle flash position, play animation

		muzzle.play("default")

		muzzle.rotation_degrees.z = randf_range(-45, 45)
		muzzle.scale = Vector3.ONE * randf_range(0.40, 0.75)
		muzzle.position = container.position - weapon.muzzle_position

		blaster_cooldown.start(weapon.cooldown)

		# Shoot the weapon, amount based on shot count

		for n in weapon.shot_count:
			raycast.target_position.x = randf_range(-weapon.spread, weapon.spread)
			raycast.target_position.y = randf_range(-weapon.spread, weapon.spread)

			raycast.force_raycast_update()

			if !raycast.is_colliding(): continue # Don't create impact when raycast didn't hit

			var collider = raycast.get_collider()

			# Hitting an enemy

			if collider.has_method("damage"):
				collider.damage(weapon.damage)

			# Creating an impact animation

			var impact = preload("res://objects/impact.tscn")
			var impact_instance = impact.instantiate()

			impact_instance.play("shot")

			get_tree().root.add_child(impact_instance)

			impact_instance.position = raycast.get_collision_point() + (raycast.get_collision_normal() / 10)
			impact_instance.look_at(camera.global_transform.origin, Vector3.UP, true)

		var knockback = random_vec2(weapon.min_knockback, weapon.max_knockback)
		# print('knockback', knockback)
		container.position.z += 0.25 # Knockback of weapon visual
		camera.rotation.x += knockback.x # Knockback of camera
		rotation.y += knockback.y
		rotation_target.x += knockback.x
		rotation_target.y += knockback.y
		movement_velocity += Vector3(0, 0, weapon.knockback) # Knockback

# Toggle between available weapons (listed in 'weapons')

func action_weapon_toggle():
	if Input.is_action_just_pressed("weapon_toggle"):
		weapon_index = wrap(weapon_index + 1, 0, weapons.size())
		initiate_change_weapon(weapon_index)

		Audio.play("sounds/weapon_change.ogg")

# Initiates the weapon changing animation (tween)

func initiate_change_weapon(index):
	weapon_index = index

	tween = get_tree().create_tween()
	tween.set_ease(Tween.EASE_OUT_IN)
	tween.tween_property(container, "position", container_offset - Vector3(0, 1, 0), 0.1)
	tween.tween_callback(change_weapon) # Changes the model

# Switches the weapon model (off-screen)

func change_weapon():
	weapon = weapons[weapon_index]

	# Step 1. Remove previous weapon model(s) from container

	for n in container.get_children():
		container.remove_child(n)

	# Step 2. Place new weapon model in container

	var weapon_model = weapon.model.instantiate()
	container.add_child(weapon_model)

	weapon_model.position = weapon.position
	weapon_model.rotation_degrees = weapon.rotation

	# Step 3. Set model to only render on layer 2 (the weapon camera)

	for child in weapon_model.find_children("*", "MeshInstance3D"):
		child.layers = 2

	# Set weapon data

	raycast.target_position = Vector3(0, 0, -1) * weapon.max_distance
	crosshair.texture = weapon.crosshair

func damage(amount):
	health -= amount
	health_updated.emit(health) # Update health on HUD

	if health < 0:
		back_to_the_start()

# Create a random knockback vector
static func random_vec2(_min: Vector2, _max: Vector2) -> Vector2:
	var _sign = -1 if randi() % 2 == 0 else 1
	return Vector2(randf_range(_min.x, _max.x), randf_range(_min.y, _max.y) * _sign)
