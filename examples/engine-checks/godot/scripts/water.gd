extends Area3D
## A body of water. The volume only says what the water is like; the swimming
## happens in the body's own movement code, where the numbers come from.

## How fast you drift down when you stop swimming, in metres per second.
@export var sink_speed: float = 0.8
## How fast Jump carries you up and Sprint carries you down.
@export var rise_speed: float = 3.5
## How fast you move through it, against walking speed on land.
@export var swim_speed: float = 3.0

var _inside: Array[Node3D] = []


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	sink_speed = float(props.get("sink_speed", sink_speed))
	rise_speed = float(props.get("rise_speed", rise_speed))
	swim_speed = float(props.get("swim_speed", swim_speed))
	body_entered.connect(_on_entered)
	body_exited.connect(_on_left)


func _on_entered(body: Node3D) -> void:
	if not body.has_method("enter_water") or _inside.has(body):
		return
	_inside.append(body)
	body.enter_water(sink_speed, rise_speed, swim_speed, surface_height())
	if get_node_or_null("/root/Audio") != null:
		get_node("/root/Audio").play("sounds/land.ogg")


func _on_left(body: Node3D) -> void:
	if not _inside.has(body):
		return
	_inside.erase(body)
	if body.has_method("exit_water"):
		body.exit_water()


## Where the top of the water is, in world space. Taken from the collision
## shape: the sides and bottom are normally skip and not in the mesh at all.
func surface_height() -> float:
	var top := -INF
	for child in get_children():
		var collider := child as CollisionShape3D
		if collider == null:
			continue
		var hull := collider.shape as ConvexPolygonShape3D
		if hull == null:
			continue
		for point in hull.points:
			top = maxf(top, collider.to_global(point).y)
	return top if top > -INF else global_position.y


## How many bodies are in the water. Used by the checks.
func occupants() -> int:
	return _inside.size()
