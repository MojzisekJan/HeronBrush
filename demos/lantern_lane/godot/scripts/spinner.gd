extends AnimatableBody3D
## Brushwork that spins for ever: fans, beacons, wheels. The pivot is the
## node's origin, which heron_brush puts on the origin brush, so a designer sets the
## axle by drawing one. Without it the middle of the bounds is used.

## Which way it turns, in map axes: x east, y north, z up.
var _axis := Vector3.UP
var _speed := 90.0
var _turning := true


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	_axis = _map_direction(str(props.get("axis", "0 0 1")))
	_speed = float(props.get("speed", 90.0))
	_turning = bool(int(props.get("start_on", 1)))


## Map axes are x east, y north, z up. Godot puts up on Y, so they rotate.
func _map_direction(text: String) -> Vector3:
	var n: PackedFloat64Array = text.split_floats(" ")
	if n.size() < 3:
		return Vector3.UP
	var v := Vector3(n[1], n[2], n[0])
	return v.normalized() if v.length() > 0.0 else Vector3.UP


func _physics_process(delta: float) -> void:
	if not _turning or is_zero_approx(_speed):
		return
	# Rotated in its own frame, so an axis given in the map stays the axis the
	# designer drew however the thing ends up turned.
	rotate_object_local(_axis, deg_to_rad(_speed) * delta)


func _on_triggered(_body: Node3D = null) -> void:
	_turning = not _turning
