extends GPUParticles3D
## Smoke, steam, dust or sparks, placed in the map. fps_smoke is this with
## a chimney's settings, fps_particles the same with every knob exposed. The
## look is built here because it all comes from the map.

## Particles alive at once. This is the "how much" knob.
@export var count: int = 24
## Seconds each one lasts.
@export var seconds: float = 3.0
## How big one is when it appears, in metres.
@export var size: float = 0.6
## What it multiplies to by the end of its life. Smoke swells as it cools.
@export var grow: float = 2.0
## How fast they leave, in metres per second.
@export var speed: float = 1.2
## Which way they go, before spread and gravity.
@export var direction: Vector3 = Vector3.UP
## How wide the stream fans out, in degrees. 180 is every direction.
@export var spread_degrees: float = 20.0
## How much they wander on the way. Smoke curls, sparks do not.
@export var swirl: float = 0.3
## What pulls on them once they are out. Smoke wants a little lift, sparks want
## real gravity, steam wants nothing.
@export var pull: Vector3 = Vector3(0.0, 0.15, 0.0)
## How quickly they slow down.
@export var drag: float = 0.4
@export var tint: Color = Color(0.78, 0.78, 0.8)
## How solid one is at its brightest. They always fade to nothing by the end.
@export var solidity: float = 0.35
## Metres the source is spread over, so it is a chimney and not a pinhole.
@export var from_size: Vector3 = Vector3(0.15, 0.15, 0.15)
## Whether they drift with the emitter or stay where they were made.
@export var carried: bool = false
## Off until something sets it off. Aim a trigger or a button at it.
@export var start_on: bool = true


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	count = int(props.get("amount", count))
	seconds = float(props.get("lifetime", seconds))
	size = float(props.get("size", size))
	grow = float(props.get("grow", grow))
	speed = float(props.get("speed", speed))
	direction = _vector(props.get("direction", ""), direction)
	spread_degrees = float(props.get("spread", spread_degrees))
	swirl = float(props.get("swirl", swirl))
	pull = _vector(props.get("gravity", ""), pull)
	drag = float(props.get("drag", drag))
	if props.get("color") is Color:
		tint = props["color"]
	solidity = float(props.get("alpha", solidity))
	from_size = _vector(props.get("from", ""), from_size)
	carried = bool(int(props.get("carried", 1 if carried else 0)))
	start_on = bool(int(props.get("start_on", 1 if start_on else 0)))
	rebuild()


## Reads "x y z" out of a map property. The FGD has no vector field type, so the
## kit writes them as text, the same as a door's open_direction.
func _vector(raw: Variant, fallback: Vector3) -> Vector3:
	if raw is Vector3:
		return raw
	var text := str(raw).strip_edges()
	if text.is_empty():
		return fallback
	var parts := text.split(" ", false)
	if parts.size() != 3:
		push_warning("particles: cannot read `%s` as three numbers" % text)
		return fallback
	return Vector3(float(parts[0]), float(parts[1]), float(parts[2]))


## Puts every setting into the particle system. Public so that a script can
## change one at run time and ask for the rest to follow.
func rebuild() -> void:
	amount = max(count, 1)
	lifetime = max(seconds, 0.05)
	local_coords = carried
	emitting = start_on
	# Particles are culled by this box, not by where they actually got to, so a
	# plume that rises out of a tight one disappears halfway up.
	var reach: float = max(speed * seconds, size * grow) + from_size.length() + 1.0
	visibility_aabb = AABB(Vector3.ONE * -reach, Vector3.ONE * reach * 2.0)

	var stuff: ParticleProcessMaterial = process_material
	if stuff == null:
		return
	stuff = stuff.duplicate()
	stuff.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	stuff.emission_box_extents = from_size
	stuff.direction = direction.normalized() if direction.length() > 0.001 else Vector3.UP
	stuff.spread = spread_degrees
	stuff.initial_velocity_min = speed * 0.7
	stuff.initial_velocity_max = speed * 1.3
	stuff.gravity = pull
	stuff.damping_min = drag * 0.5
	stuff.damping_max = drag
	stuff.scale_min = size * 0.75
	stuff.scale_max = size * 1.25
	stuff.turbulence_enabled = swirl > 0.001
	stuff.turbulence_noise_strength = swirl * 3.0
	stuff.turbulence_noise_scale = 2.0
	stuff.turbulence_noise_speed = Vector3(0.2, 0.2, 0.2)

	# Grows over its life, so one puff reads as spreading out rather than as a
	# sprite sliding across the screen.
	var swell := Curve.new()
	swell.add_point(Vector2(0.0, 1.0))
	swell.add_point(Vector2(1.0, max(grow, 0.01)))
	var swell_texture := CurveTexture.new()
	swell_texture.curve = swell
	stuff.scale_curve = swell_texture

	# And fades. Ending on a hard edge is what makes cheap particles look cheap.
	var fade := Gradient.new()
	fade.set_color(0, Color(tint.r, tint.g, tint.b, 0.0))
	fade.set_color(1, Color(tint.r, tint.g, tint.b, 0.0))
	fade.set_offset(0, 0.0)
	fade.set_offset(1, 1.0)
	fade.add_point(0.15, Color(tint.r, tint.g, tint.b, solidity))
	fade.add_point(0.6, Color(tint.r, tint.g, tint.b, solidity * 0.7))
	var fade_texture := GradientTexture1D.new()
	fade_texture.gradient = fade
	stuff.color_ramp = fade_texture

	process_material = stuff


## Turns it on, or off again if it was already going. Wired the same way as the
## doors, so a trigger or a button can work it with nothing new to learn.
func _on_triggered(_body: Node3D = null) -> void:
	emitting = not emitting
