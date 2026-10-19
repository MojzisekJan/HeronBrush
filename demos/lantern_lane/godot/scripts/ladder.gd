extends Area3D
## A volume you climb. Where you go is where you look, the way Quake and
## Half-Life did it, so there is no direction to aim wrong in the editor.

## Metres a second. Zero hands the decision back to the player.
@export var speed: float = 0.0

var _inside: Array[Node3D] = []


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	speed = float(props.get("speed", speed))
	body_entered.connect(_on_entered)
	body_exited.connect(_on_left)


func _on_entered(body: Node3D) -> void:
	if not body.has_method("enter_ladder") or _inside.has(body):
		return
	_inside.append(body)
	body.enter_ladder(speed)


func _on_left(body: Node3D) -> void:
	if not _inside.has(body):
		return
	_inside.erase(body)
	if body.has_method("exit_ladder"):
		body.exit_ladder()


## How many bodies are on it. Used by the checks.
func occupants() -> int:
	return _inside.size()
