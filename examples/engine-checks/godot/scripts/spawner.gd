extends Node3D
## Puts enemies into the level when something sets it off, using the same
## targeting the doors use.

@export var scene: PackedScene = preload("res://objects/enemy.tscn")
## How many arrive in one wave.
@export var count: int = 3
## Seconds between them, so they do not all appear at once.
@export var interval: float = 0.8
## Metres they are scattered over, to stop them stacking up.
@export var spread: float = 2.0

var _left := 0
var _timer := 0.0
var _spawned: Array[Node] = []


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	count = int(props.get("count", count))
	interval = float(props.get("interval", interval))
	spread = float(props.get("spread", spread))


func _process(delta: float) -> void:
	if _left <= 0:
		return
	_timer -= delta
	if _timer <= 0.0:
		_spawn_one()
		_timer = interval


## Called by whatever targets this spawner.
func _on_triggered(body: Node3D = null) -> void:
	if body != null and not body is CharacterBody3D:
		return
	if _left > 0:
		return  # a wave is already arriving
	_left = count
	_timer = 0.0


func _spawn_one() -> void:
	var index := count - _left
	_left -= 1

	var enemy: Node3D = scene.instantiate()

	# The place has to be set before the node enters the tree: the kit's enemy
	# reads its position in `_ready` and holds itself there, so anything moved
	# afterwards springs back and the wave ends up in one spot.
	var angle := TAU * (float(index) + 0.5) / float(max(count, 1))
	enemy.position = position + Vector3(cos(angle), 0.0, sin(angle)) * spread
	enemy.add_to_group("enemies")
	if "player" in enemy:
		enemy.player = get_tree().get_first_node_in_group("player")

	# Added to the parent rather than to the spawner, so the spawner's own
	# position is not applied on top of the one just worked out.
	get_parent().add_child(enemy)
	_spawned.append(enemy)


## The enemies this spawner has put out and that are still alive.
func spawned() -> Array:
	return _spawned.filter(func(e): return is_instance_valid(e))


## How many this spawner has put out. Used by the checks.
func spawned_count() -> int:
	return spawned().size()
