extends Area3D
var used := false
func _ready() -> void:
	body_entered.connect(func(body: Node3D):
		if not body.is_in_group("relay_player") or used:
			return
		var props: Dictionary = get_meta("properties", {})
		var game := get_tree().get_first_node_in_group("relay_game")
		if str(props.get("kind", "exit")) == "hazard":
			game.respawn()
		else:
			used = game.try_exit(str(props.get("required", "")), str(props.get("next_level", ""))))
