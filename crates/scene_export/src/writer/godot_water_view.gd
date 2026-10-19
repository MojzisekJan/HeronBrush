extends Node
## Put this under the game's Camera3D. While the camera is in water, it sees
## through the water's colour as fog; out of it, the camera gets back the
## environment it had. A game with its own underwater effect leaves this out and
## calls `find` itself.


## The volume `world` is in and how deep, as [water, depth], or [] when it is
## in none. Where volumes overlap the higher priority wins, then the deeper.
static func find(tree: SceneTree, world: Vector3) -> Array:
	var best = null
	var best_depth := -1.0
	for water in tree.get_nodes_in_group("heron_water"):
		if not water.has_method("sample") or not water.can_process():
			continue
		var depth: float = water.sample(world)
		if depth < 0.0:
			continue
		if best == null or water.priority() > best.priority() \
				or (water.priority() == best.priority() and depth > best_depth):
			best = water
			best_depth = depth
	return [] if best == null else [best, best_depth]


var _active := false
var _saved: Environment
var _environment: Environment


func _process(_delta: float) -> void:
	var camera := get_parent() as Camera3D
	if camera == null:
		return
	var found := find(get_tree(), camera.global_position)
	if found.is_empty():
		_restore(camera)
		return
	if not _active:
		_saved = camera.environment
		var base := camera.environment
		if base == null and camera.get_world_3d() != null:
			base = camera.get_world_3d().environment
			if base == null:
				base = camera.get_world_3d().fallback_environment
		_environment = base.duplicate() if base != null else Environment.new()
		_active = true
	var tint: Color = found[0].tint()
	_environment.fog_enabled = true
	_environment.fog_mode = Environment.FOG_MODE_EXPONENTIAL
	_environment.fog_light_color = Color(tint.r, tint.g, tint.b)
	_environment.fog_light_energy = 1.0
	_environment.fog_density = found[0].fog_density()
	_environment.fog_sky_affect = 1.0
	camera.environment = _environment


func _exit_tree() -> void:
	_restore(get_parent() as Camera3D)


func _restore(camera: Camera3D) -> void:
	if not _active:
		return
	if camera != null:
		camera.environment = _saved
	_active = false
