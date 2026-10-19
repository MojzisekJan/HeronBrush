extends AnimatableBody3D
## Brushwork that slides when something tells it to: doors, lifts, platforms.
## Settings come from `properties`, and triggers reach it through a real signal
## connection the build wires up.

## Name of the key this needs, or empty for a door anyone can open.
var required_key: String = ""

var _warned_about_keys := false

var _closed_at: Vector3
var _open_at: Vector3
var _speed: float = 3.0
var _wait: float = 3.0
var _held: float = 0.0
var _opening: bool = false


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	required_key = str(props.get("key", ""))
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


## Called by whatever targets this, and only something that walks counts.
## Without the guard a trigger touching its own door sees the door enter it.
func _on_triggered(body: Node3D = null) -> void:
	if body != null and not body is CharacterBody3D:
		return
	if not required_key.is_empty() and not _key_in_hand():
		if get_node_or_null("/root/Audio") != null:
			get_node("/root/Audio").play("sounds/weapon_change.ogg")
		return
	_opening = true
	_held = 0.0


## True while the door is not sitting shut. Handy in tests.
func is_moving_or_open() -> bool:
	return _opening or not position.is_equal_approx(_closed_at)


## Asks whoever is running the level whether the player has the key. `owner` is
## the map's own root, so this walks up to find whoever is keeping track.
func _key_in_hand() -> bool:
	var node: Node = self
	while node != null:
		if node.has_method("has_key"):
			return node.has_key(required_key)
		node = node.get_parent()

	# Nobody is keeping track. Staying shut is the safer failure: a door that
	# never opens is noticed, one that opens for free is not.
	if not _warned_about_keys:
		_warned_about_keys = true
		push_warning("%s asks for the '%s' key but nothing in the scene tracks keys, so it stays shut"
			% [name, required_key])
	return false
