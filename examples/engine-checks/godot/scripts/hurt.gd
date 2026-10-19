extends Area3D
## Lava, spikes, acid. Damage comes in bites rather than every frame, the way
## Quake did it, so you hear and see each one.

## How much a second, spread over the bites.
@export var damage: float = 20.0
## Seconds between bites.
@export var interval: float = 0.5
## Kill outright, whatever the health.
@export var instant: bool = false
## Shown the first time it bites.
@export var message: String = ""

var _inside: Array[Node3D] = []
var _timer := 0.0
var _said := false


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	damage = float(props.get("damage", damage))
	interval = maxf(float(props.get("interval", interval)), 0.05)
	instant = bool(int(props.get("instant", 0)))
	message = str(props.get("message", message))
	body_entered.connect(_on_entered)
	body_exited.connect(_on_left)


func _on_entered(body: Node3D) -> void:
	if not body.has_method("damage") or _inside.has(body):
		return
	_inside.append(body)
	# The first bite lands at once. Walking into lava and having half a second
	# of nothing happen reads as the lava being broken.
	_timer = 0.0


func _on_left(body: Node3D) -> void:
	_inside.erase(body)


func _process(delta: float) -> void:
	if _inside.is_empty():
		return
	_timer -= delta
	if _timer > 0.0:
		return
	_timer = interval

	if not _said and not message.is_empty():
		_said = true
		var runner: Node = _runner()
		if runner != null and runner.has_method("say"):
			runner.say(message)

	for body in _inside:
		if not is_instance_valid(body):
			continue
		body.damage(9999 if instant else int(round(damage * interval)))


func _runner() -> Node:
	var node: Node = get_parent()
	while node != null:
		if node.has_method("say"):
			return node
		node = node.get_parent()
	return null
