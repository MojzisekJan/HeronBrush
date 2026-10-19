extends Node3D
@export var station_id := ""
@export var cost := 2
@export var target := ""
var active := false
func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	station_id = str(props.get("station_id", station_id))
	cost = int(props.get("cost", cost))
	target = str(props.get("target", target))
func power_on() -> void:
	active = true
	var material := StandardMaterial3D.new()
	material.albedo_color = Color(0.2, 1, 0.7)
	material.emission_enabled = true
	material.emission = material.albedo_color
	$Lamp.material_override = material
