extends Area3D
## A key. Walk through it to carry it, then a door that asks for it will open.

signal taken(key_name: String)

@export var key_name: String = "red"

var _taken := false
var _time := 0.0
var _home := Vector3.ZERO


func _ready() -> void:
	_home = position
	var props: Dictionary = get_meta("properties", {})
	key_name = str(props.get("key", key_name))
	if props.has("color"):
		_tint(props["color"])
	body_entered.connect(_on_body_entered)


func _process(delta: float) -> void:
	if _taken:
		return
	_time += delta
	rotate_y(delta * 1.6)
	position.y = _home.y + sin(_time * 2.5) * 0.1


func _on_body_entered(body: Node3D) -> void:
	if _taken or not body is CharacterBody3D:
		return
	_taken = true
	remove_from_group("keys")
	taken.emit(key_name)
	if get_node_or_null("/root/Audio") != null:
		get_node("/root/Audio").play("sounds/weapon_change.ogg")
	queue_free()


func _tint(raw: Variant) -> void:
	var colour: Color = raw if raw is Color else Color(1, 0.3, 0.3)
	var material := $mesh.get_surface_override_material(0) as StandardMaterial3D
	if material == null:
		return
	material = material.duplicate()
	material.albedo_color = colour
	material.emission = colour
	$mesh.set_surface_override_material(0, material)
