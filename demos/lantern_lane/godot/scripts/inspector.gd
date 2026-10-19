extends Node
class_name Inspector
## Hands the view over to a free camera and back, on F3. The player is left
## standing where they were, and the level keeps running underneath.

@export var map_units: float = 32.0

var player: Node3D = null
var hud: CanvasLayer = null
var spectator: Spectator = null
## What the player's own collision layer was. While the view is detached they
## come off every layer, or enemies keep shooting a body that cannot answer.
var player_layer := 0


func _ready() -> void:
	player = get_parent().get_node_or_null("Player")
	hud = get_parent().get_node_or_null("HUD")


func _process(_delta: float) -> void:
	if Input.is_action_just_pressed("spectate"):
		if spectator == null:
			detach()
		else:
			reattach()


func detach() -> void:
	if player == null:
		return
	var camera: Camera3D = player.get_node_or_null("Head/Camera")
	if camera == null:
		return
	spectator = Spectator.new()
	spectator.name = "Spectator"
	spectator.map_units = map_units
	get_parent().add_child(spectator)
	spectator.global_transform = camera.global_transform
	spectator.fov = camera.fov

	player.set_physics_process(false)
	player.set_process(false)
	player.set_process_input(false)
	player_layer = player.collision_layer
	player.collision_layer = 0
	# The weapon is held in front of the player's camera, so from any other
	# camera it hangs in the air across half the view.
	player.visible = false
	if hud != null:
		hud.visible = false


func reattach() -> void:
	if spectator != null:
		spectator.queue_free()
		spectator = null
	if player == null:
		return
	var camera: Camera3D = player.get_node_or_null("Head/Camera")
	if camera != null:
		camera.current = true
	player.set_physics_process(true)
	player.set_process(true)
	player.set_process_input(true)
	player.collision_layer = player_layer
	player.visible = true
	if hud != null:
		hud.visible = true
