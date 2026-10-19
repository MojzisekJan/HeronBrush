extends AnimatableBody3D
## A door drawn in the map.
##
## Settings come from the `properties` metadata that heron_brush writes onto the node,
## so the same script serves every door in the map. Triggers and buttons reach
## it through `_on_triggered`, which the build wires up as a real signal
## connection; nothing here looks anything up by name.

var _closed_at: Vector3
var _open_at: Vector3
var _speed: float = 3.0
var _wait: float = 3.0
var _held: float = 0.0
var _opening: bool = false


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	_speed = float(props.get("speed", 100.0)) / 32.0
	_wait = float(props.get("wait", 3.0))
	_closed_at = position
	var travel: float = float(props.get("travel", 128.0)) / 32.0
	_open_at = _closed_at + _map_direction(str(props.get("open_direction", "0 0 1"))) * travel


## Map axes are x east, y north, z up. Godot puts up on Y, so they rotate.
func _map_direction(text: String) -> Vector3:
	var n: PackedFloat64Array = text.split_floats(" ")
	if n.size() < 3:
		return Vector3.UP
	var v := Vector3(n[1], n[2], n[0])
	return v.normalized() if v.length() > 0.0 else Vector3.UP


func _physics_process(delta: float) -> void:
	var goal: Vector3 = _open_at if _opening else _closed_at
	position = position.move_toward(goal, _speed * delta)

	if _opening and position.is_equal_approx(_open_at):
		_held += delta
		if _held >= _wait:
			_opening = false
			_held = 0.0


## Called by whatever targets this. The body is whatever walked into the
## trigger, and only something that walks counts.
##
## Without this guard the geometry sets itself off: a door is a physics body on
## the same collision layer as the player, so a trigger volume that so much as
## touches its own door sees the door enter and opens it on the first frame.
func _on_triggered(body: Node3D = null) -> void:
	if body != null and not body is CharacterBody3D:
		return
	_opening = true
	_held = 0.0


## True while the door is not sitting shut. Handy in tests.
func is_moving_or_open() -> bool:
	return _opening or not position.is_equal_approx(_closed_at)
