extends StaticBody3D
## A barrel that goes off when it is shot. Anything in the blast that can be
## damaged is, barrels included, so they set each other off.

## How much shooting it takes.
@export var health: int = 30
## Damage right at the centre. It falls off to nothing at the edge of the blast.
@export var blast: int = 60
## How far the blast reaches, in metres.
@export var radius: float = 4.0
## How hard it throws whatever it catches.
@export var push: float = 9.0
## Seconds between the shot that killed it and the bang.
@export var fuse: float = 0.0

var _going_off := false


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	health = int(props.get("health", health))
	blast = int(props.get("damage", blast))
	radius = float(props.get("radius", radius))
	push = float(props.get("push", push))
	fuse = float(props.get("fuse", fuse))
	add_to_group("barrels")


## Shot, or caught in another barrel's blast. The weapon calls this on whatever
## it hits, so nothing had to be told that barrels exist.
func damage(amount: int) -> void:
	if _going_off:
		return
	health -= amount
	if health > 0:
		return
	_going_off = true
	if fuse > 0.0:
		await get_tree().create_timer(fuse).timeout
	explode()


func explode() -> void:
	var here: Vector3 = global_position + Vector3.UP * 0.8
	Audio.play("sounds/enemy_destroy.ogg")
	_show_blast(here)

	# Walls are not checked for: a barrel round a corner still going off beats one
	# that quietly does nothing because a pillar clipped the query.
	var ball := SphereShape3D.new()
	ball.radius = radius
	var query := PhysicsShapeQueryParameters3D.new()
	query.shape = ball
	query.transform = Transform3D(Basis(), here)
	query.collide_with_bodies = true
	query.collide_with_areas = false
	query.exclude = [get_rid()]

	for hit in get_world_3d().direct_space_state.intersect_shape(query, 32):
		var body = hit["collider"]
		if body == null or not is_instance_valid(body):
			continue
		var away: Vector3 = (body.global_position - here)
		var reach: float = away.length()
		# Straight down the middle hurts most, the edge of the blast not at all.
		var share: float = clampf(1.0 - reach / max(radius, 0.01), 0.0, 1.0)
		if share <= 0.0:
			continue
		if body.has_method("damage"):
			body.damage(int(round(float(blast) * share)))
		_throw(body, away.normalized() if reach > 0.01 else Vector3.UP, share)

	queue_free()


## Throws a body away from the blast. The kit's player keeps its vertical speed
## in a separate number, so up and along have to be handed over separately.
func _throw(body: Node3D, away: Vector3, share: float) -> void:
	var force: float = push * share
	if "gravity" in body and "velocity" in body:
		body.velocity += Vector3(away.x, 0.0, away.z) * force
		body.gravity -= (away.y + 0.5) * force
	elif body is RigidBody3D:
		body.apply_impulse(away * force * body.mass)


func _show_blast(here: Vector3) -> void:
	var burst: GPUParticles3D = preload("res://objects/particles.tscn").instantiate()
	burst.count = 48
	burst.seconds = 0.9
	burst.size = 0.35
	burst.grow = 2.5
	burst.speed = radius * 2.0
	burst.spread_degrees = 180.0
	burst.pull = Vector3(0.0, -4.0, 0.0)
	burst.drag = 1.5
	burst.tint = Color(1.0, 0.65, 0.2)
	burst.solidity = 1.0
	burst.from_size = Vector3(0.2, 0.2, 0.2)
	burst.start_on = true
	get_parent().add_child(burst)
	burst.global_position = here
	burst.one_shot = true
	burst.explosiveness = 1.0
	burst.emitting = true
	# Freed once the last puff has gone, so nothing is left running in the level.
	get_tree().create_timer(burst.seconds + 0.5).timeout.connect(burst.queue_free)
