extends Area3D
## Something the player picks up by walking through it, placed in the map
## as `fps_pickup`. Its worth comes from the map, through `properties`.

signal collected(value: int)

@export var value: int = 1
## Degrees per second.
@export var spin_speed: float = 120.0
## How far it drifts up and down, in metres.
@export var bob_height: float = 0.12

var _home := Vector3.ZERO
var _time := 0.0
var _taken := false


func _ready() -> void:
	_home = position
	var props: Dictionary = get_meta("properties", {})
	value = int(props.get("value", value))
	body_entered.connect(_on_body_entered)


func _process(delta: float) -> void:
	if _taken:
		return
	_time += delta
	rotate_y(deg_to_rad(spin_speed) * delta)
	position.y = _home.y + sin(_time * 3.0) * bob_height


## Anything that walks takes it. Enemies are plain Node3D, so they cannot.
func _on_body_entered(body: Node3D) -> void:
	if _taken or not body is CharacterBody3D:
		return
	_taken = true
	# Leave the group before announcing it: queue_free only takes effect at the end
	# of the frame, which is too late for anyone counting.
	remove_from_group("collectibles")
	collected.emit(value)
	if Engine.has_singleton("Audio") or get_node_or_null("/root/Audio") != null:
		get_node("/root/Audio").play("sounds/pickup.wav")
	queue_free()
