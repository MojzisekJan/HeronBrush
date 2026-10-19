extends Area3D
## Walk through one and you come back here instead of at the start.

signal reached(checkpoint: Area3D)

var used := false


func _ready() -> void:
	body_entered.connect(_on_body_entered)


func _on_body_entered(body: Node3D) -> void:
	if used or not body is CharacterBody3D:
		return
	used = true
	reached.emit(self)
	if get_node_or_null("/root/Audio") != null:
		get_node("/root/Audio").play("sounds/pickup.wav")
