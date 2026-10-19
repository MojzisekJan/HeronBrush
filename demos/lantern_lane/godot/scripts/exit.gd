extends Area3D
## The way out of a level. It carries the name of the next map, so a campaign
## is built in the maps and nothing in Godot knows the order.

## A map name such as `arena`, or a full `res://` path. Empty finishes here.
@export var next_map: String = ""
## Shown on screen as it happens.
@export var message: String = "Level done"
## The name of a key the player has to be carrying, if any.
@export var needs: String = ""

var _used := false


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	next_map = str(props.get("next_map", next_map))
	message = str(props.get("message", message))
	needs = str(props.get("needs", needs))
	body_entered.connect(_on_body_entered)


func _on_body_entered(body: Node3D) -> void:
	if _used or not body is CharacterBody3D:
		return
	var runner: Node = _runner()
	if runner == null:
		push_error("fps_exit: nothing here knows how to finish a level")
		return

	# A locked exit is how a level says "find the key first". The door entity
	# could do the same, but a door has to be somewhere; an exit is the end.
	if not needs.is_empty() and runner.has_method("has_key") and not runner.has_key(needs):
		if runner.has_method("say"):
			runner.say("The way out needs the %s key" % needs)
		return

	_used = true
	runner.finish_level(next_map, message)


## Whoever is running the level. Same idea as the player asking for a respawn:
## the entity says what happened, the level decides what that means.
func _runner() -> Node:
	var node: Node = get_parent()
	while node != null:
		if node.has_method("finish_level"):
			return node
		node = node.get_parent()
	return null
