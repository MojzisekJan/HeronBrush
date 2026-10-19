extends Area3D
@export var cell_id := ""
@export var value := 1
func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	cell_id = str(props.get("cell_id", cell_id))
	value = int(props.get("value", value))
	body_entered.connect(func(body: Node3D):
		if body.is_in_group("relay_player"):
			get_tree().get_first_node_in_group("relay_game").collect(self))
func _process(dt: float) -> void:
	$Mesh.rotation.y += dt
