extends AnimatableBody3D
## Brushwork that slides back and forth for ever: a piston, a crusher, a moving
## platform. Unlike a door it never waits to be told, so you time your jump.

var _home: Vector3
var _away: Vector3
var _speed := 2.0
var _wait := 0.5
var _held := 0.0
var _outbound := true
var _moving := true


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	# Map units throughout, the same as the doors, so a designer measures
	# everything in the grid they are drawing on.
	_speed = float(props.get("speed", 64.0)) / 32.0
	_wait = float(props.get("wait", 0.5))
	_moving = bool(int(props.get("start_on", 1)))
	_home = position
	var travel: float = float(props.get("travel", 128.0)) / 32.0
	_away = _home + _map_direction(str(props.get("direction", "1 0 0"))) * travel


func _map_direction(text: String) -> Vector3:
	var n: PackedFloat64Array = text.split_floats(" ")
	if n.size() < 3:
		return Vector3.RIGHT
	var v := Vector3(n[1], n[2], n[0])
	return v.normalized() if v.length() > 0.0 else Vector3.RIGHT


func _physics_process(delta: float) -> void:
	if not _moving:
		return
	if _held > 0.0:
		_held -= delta
		return

	var goal: Vector3 = _away if _outbound else _home
	position = position.move_toward(goal, _speed * delta)
	if position.is_equal_approx(goal):
		_outbound = not _outbound
		_held = _wait


func _on_triggered(_body: Node3D = null) -> void:
	_moving = not _moving
