extends Area3D
## Launches whoever steps on it. The player keeps its vertical motion in one
## scalar, so a pad is a jump with a bigger number.

## How hard it throws you. The player's own jump is 8.
@export var strength: float = 15.0


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	strength = float(props.get("strength", strength))
	body_entered.connect(_on_body_entered)


func _on_body_entered(body: Node3D) -> void:
	if not body is CharacterBody3D:
		return
	if "gravity" in body:
		body.gravity = -strength
	else:
		body.velocity.y = strength
	if get_node_or_null("/root/Audio") != null:
		get_node("/root/Audio").play("sounds/jump_b.ogg")
