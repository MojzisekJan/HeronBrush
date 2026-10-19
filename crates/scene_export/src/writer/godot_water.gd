extends Node
## One water volume, under the Area3D whose shapes are its closed brushes. The
## volume is a sensor: nothing here moves a body. A player asks `sample` where it
## is every step and, while that says it is in the water, takes its velocity from
## `velocity`. heron_water_view.gd, beside the level, finds the volume a point is
## in among several and tints a camera that is under water.

## The exported volume: convex planes local to the Area3D, in metres.
@export var data_json := ""
var data: Dictionary


func _ready() -> void:
	var parsed = JSON.parse_string(data_json)
	if typeof(parsed) != TYPE_DICTIONARY or int(parsed.get("version", 0)) != 1:
		push_error("%s: unsupported water payload" % get_path())
		return
	data = parsed


## How far below the surface `world` is, in metres, or -1 outside the water.
## The planes are tested exactly, so a point in the corner of the volume's box
## that the brushes do not fill is outside.
func sample(world: Vector3) -> float:
	var body := get_parent() as Node3D
	if body == null or data.is_empty():
		return -1.0
	var p := body.global_transform.affine_inverse() * world
	var scale_y := absf(body.global_transform.basis.get_scale().y)
	var depth := -1.0
	for volume in data.volumes:
		var planes: Array = volume.planes
		if planes.size() < 4:
			continue
		var inside := true
		for plane in planes:
			if plane.x * p.x + plane.y * p.y + plane.z * p.z > plane.d + 1e-5:
				inside = false
				break
		if inside:
			depth = maxf(depth, (float(volume.surface) - p.y) * scale_y)
	return depth


## Swimming: `walk` steers in the horizontal plane (its length is capped at 1),
## `rise` and `dive` move up and down, and with neither the body sinks slowly.
## The current velocity eases towards that at the volume's drag.
func velocity(current: Vector3, walk: Vector3, rise: bool, dive: bool, delta: float) -> Vector3:
	var wish := walk.limit_length(1.0) * float(data.swim_speed)
	wish.y = 0.0
	if rise:
		wish.y += float(data.rise_speed)
	if dive:
		wish.y -= float(data.rise_speed)
	if not rise and not dive:
		wish.y -= float(data.sink_speed)
	return current.lerp(wish, 1.0 - exp(-float(data.drag) * delta))


## Higher wins where volumes overlap.
func priority() -> int:
	return int(data.get("priority", 0))


## The water's colour, with the surface's opacity as alpha.
func tint() -> Color:
	var c: Array = data.get("color", [0.05, 0.38, 0.48])
	return Color(c[0], c[1], c[2], float(data.get("opacity", 0.55)))


func fog_density() -> float:
	return float(data.get("fog_density", 0.18))
