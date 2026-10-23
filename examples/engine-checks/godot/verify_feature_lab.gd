extends SceneTree
## Runs the feature lab's Godot export in the engine, station by station.
## verify_feature_lab.py copies this into the export folder and runs it there:
##   godot --headless --path <export> --script res://verify_feature_lab.gd
## With `-- --shots=<dir>` and a real renderer instead of --headless, it saves
## pictures of each station into <dir> instead of checking.
## Every check prints `PASS <feature>: <what>` or `FAIL <feature>: <what>`; the last
## line is `FEATURE LAB: <n> failures`.

const SCALE := 1.0 / 32.0
## U and V span of each fl_textures face over its 64 unit side (64 pixel images).
const SPANS := {
	"wood": Vector2(1, 1), "metal": Vector2(1, 1), "crate": Vector2(sqrt(2.0), sqrt(2.0)),
	"wall": Vector2(2, 0.5), "dirt": Vector2(0.5, 0.5), "hazard": Vector2(1, 1),
}
const OPAQUE_COLOURS := [Color8(220, 30, 40), Color8(30, 200, 60), Color8(40, 60, 230),
	Color8(240, 220, 30), Color8(255, 255, 255)]
const GRADIENT_STOPS := [
	[Color8(255, 0, 0), Color8(0, 0, 255)],
	[Color8(255, 255, 0), Color8(0, 255, 0), Color8(0, 0, 0)],
	[Color8(255, 255, 255), Color8(64, 0, 128)],
	[Color8(0, 0, 0), Color8(255, 255, 255)],
]
## Roughness, metallic and emission of the four fl_surface overrides.
const SURFACES := [Vector3(0.2, 0.9, 0.0), Vector3(0.9, 0.0, 0.0), Vector3(0.65, 0.0, 3.0),
	Vector3(0.05, 1.0, 0.0)]

var failures := 0
var level: Node3D
var shots := ""


func check(ok: bool, feature: String, label: String) -> void:
	print(("PASS " if ok else "FAIL ") + feature + ": " + label)
	if not ok:
		failures += 1


func _initialize() -> void:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--shots="):
			shots = argument.trim_prefix("--shots=")
	run.call_deferred()


func station(targetname: String) -> Node3D:
	for node in level.get_children():
		if node.get_meta("properties", {}).get("targetname", "") == targetname:
			return node
	return null


func mesh_of(node: Node) -> MeshInstance3D:
	for child in node.get_children():
		if child is MeshInstance3D:
			return child
	return null


## Map units to Godot metres: map X is Godot Z, map Y is Godot X, map Z is Godot Y.
func to_godot(p: Vector3) -> Vector3:
	return Vector3(p.y, p.z, p.x) * SCALE


func to_map(p: Vector3) -> Vector3:
	return Vector3(p.z, p.x, p.y) / SCALE


func image_of(texture: Texture2D) -> Image:
	if texture == null:
		return null
	var image := texture.get_image()
	if image != null and image.is_compressed():
		image.decompress()
	if image != null:
		image.convert(Image.FORMAT_RGBA8)
	return image


func source_image(res_path: String) -> Image:
	var image := Image.load_from_file(ProjectSettings.globalize_path(res_path))
	if image != null:
		image.convert(Image.FORMAT_RGBA8)
	return image


func same_pixels(a: Image, b: Image) -> bool:
	return a != null and b != null and a.get_size() == b.get_size() and a.get_data() == b.get_data()


func colour_near(a: Color, b: Color, tolerance: float = 3.0 / 255.0) -> bool:
	return absf(a.r - b.r) <= tolerance and absf(a.g - b.g) <= tolerance and absf(a.b - b.b) <= tolerance


func image_has(image: Image, colour: Color, tolerance: float = 3.0 / 255.0) -> bool:
	for y in image.get_height():
		for x in image.get_width():
			if colour_near(image.get_pixel(x, y), colour, tolerance):
				return true
	return false


## Triangles of one surface as [positions, uvs, normal] in the mesh's own frame.
func triangles(mesh: Mesh, surface: int) -> Array:
	var arrays := mesh.surface_get_arrays(surface)
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV] if arrays[Mesh.ARRAY_TEX_UV] != null else PackedVector2Array()
	var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX] if arrays[Mesh.ARRAY_INDEX] != null else PackedInt32Array()
	var order := indices if indices.size() > 0 else PackedInt32Array(range(vertices.size()))
	var out := []
	for i in range(0, order.size(), 3):
		var p := [vertices[order[i]], vertices[order[i + 1]], vertices[order[i + 2]]]
		var t := []
		if uvs.size() > 0:
			t = [uvs[order[i]], uvs[order[i + 1]], uvs[order[i + 2]]]
		# Godot's front faces wind clockwise.
		var n: Vector3 = (p[2] - p[0]).cross(p[1] - p[0]).normalized()
		out.append([p, t, n])
	return out


## Distinct axis directions a set of triangles faces.
func sides(tris: Array) -> Array:
	var found := []
	for tri in tris:
		var n: Vector3 = tri[2]
		var axis := Vector3(roundf(n.x), roundf(n.y), roundf(n.z))
		if not found.has(axis):
			found.append(axis)
	return found


## How U (or V) changes along the triangle, as a world-space gradient.
func uv_gradient(tri: Array, component: int) -> Vector3:
	var p: Array = tri[0]
	var t: Array = tri[1]
	var e1: Vector3 = p[1] - p[0]
	var e2: Vector3 = p[2] - p[0]
	var d1: float = t[1][component] - t[0][component]
	var d2: float = t[2][component] - t[0][component]
	# Solve g . e1 = d1, g . e2 = d2 with g in the triangle's plane.
	var a := e1.dot(e1)
	var b := e1.dot(e2)
	var c := e2.dot(e2)
	var det := a * c - b * b
	var s := (d1 * c - d2 * b) / det
	var r := (d2 * a - d1 * b) / det
	return e1 * s + e2 * r


func run() -> void:
	var packed: PackedScene = load("res://feature_lab.tscn")
	check(packed != null, "Import", "feature_lab.tscn loads")
	if packed == null:
		finish()
		return
	level = packed.instantiate()
	root.add_child(level)
	await physics_frame
	await physics_frame
	if not shots.is_empty():
		await take_shots()
		finish()
		return
	check_import()
	check_textures()
	check_colours()
	check_gradients()
	check_surfaces()
	check_normal_map()
	check_hidden_and_skip()
	check_collision()
	await check_animation()
	await check_playback()
	await check_trigger_and_button()
	await check_water()
	check_sky()
	check_lights()
	check_models()
	check_properties()
	finish()


func finish() -> void:
	print("FEATURE LAB: %d failures" % failures)
	quit(1 if failures > 0 else 0)


func check_import() -> void:
	var meshes := level.find_children("*", "MeshInstance3D", true, false)
	var missing := []
	var textures := {}
	for instance: MeshInstance3D in meshes:
		if instance.mesh == null:
			missing.append(str(instance.get_path()))
			continue
		for index in instance.mesh.get_surface_count():
			var material := instance.get_active_material(index)
			if material is StandardMaterial3D:
				for property in ["albedo_texture", "normal_texture", "roughness_texture", "ao_texture", "emission_texture"]:
					var texture: Texture2D = material.get(property)
					if texture != null:
						textures[texture.resource_path] = texture
	check(missing.is_empty() and meshes.size() == 15, "Import",
		"%d MeshInstance3D nodes, every OBJ mesh imported %s" % [meshes.size(), missing])
	var relative := 0
	var unloaded := []
	for path in textures:
		if path.begins_with("res://textures/"):
			relative += 1
		if textures[path].get_width() == 0:
			unloaded.append(path)
	# The scene names them `textures/<name>.png`, relative to itself.
	check(relative >= 9 and unloaded.is_empty(), "Import",
		"%d textures load, %d of them through the scene's relative textures/ paths" % [textures.size(), relative])


func check_textures() -> void:
	var node := station("fl_textures")
	var instance := mesh_of(node)
	var problems := []
	var gradients := {}
	var corners := {}
	for index in instance.mesh.get_surface_count():
		var material := instance.get_active_material(index) as StandardMaterial3D
		var name := material.albedo_texture.resource_path.get_file().get_basename()
		var res_path := "res://textures/%s.png" % name
		if material.albedo_texture.resource_path != res_path:
			problems.append("%s: albedo %s" % [name, material.albedo_texture.resource_path])
		if not same_pixels(image_of(material.albedo_texture), source_image(res_path)):
			problems.append("%s: imported pixels differ from the source image" % name)
		if not material.texture_repeat:
			problems.append("%s does not repeat" % name)
		var tris := triangles(instance.mesh, index)
		if name == "grass":
			for tri in tris:
				if tri[2].y > 0.99:
					gradients["grass_top"] = uv_gradient(tri, 0)
			continue
		var lo := Vector2(INF, INF)
		var hi := Vector2(-INF, -INF)
		for tri in tris:
			for uv: Vector2 in tri[1]:
				lo = lo.min(uv)
				hi = hi.max(uv)
		var span := hi - lo
		if not span.is_equal_approx(SPANS[name]) and (span - SPANS[name]).length() > 0.02:
			problems.append("%s: UV span %s, want %s" % [name, span, SPANS[name]])
		gradients[name] = uv_gradient(tris[0], 0)
		gradients[name + "_v"] = uv_gradient(tris[0], 1)
		corners[name] = tris
	# Image up is world up on an unrotated wall: V (down the image) falls as Y rises.
	for name in ["wood", "metal", "wall"]:
		if gradients[name + "_v"].y >= 0.0:
			problems.append("%s: V grows upwards, the image is upside down" % name)
	# The crate face is turned 45 degrees: U runs diagonally.
	var crate: Vector3 = gradients["crate"].normalized()
	if absf(absf(crate.y) - sqrt(0.5)) > 0.01:
		problems.append("crate: U gradient %s is not at 45 degrees" % crate)
	# The dirt top is turned 90 degrees against the plain grass top.
	if not gradients.has("grass_top") or absf(gradients["grass_top"].normalized().dot(gradients["dirt"].normalized())) > 0.01:
		problems.append("dirt: U runs along the same axis as the unrotated grass top")
	# Metal (offset 16 8) against wood on the opposite side: shift of 1/4 and 1/8 of a tile.
	var shifts := []
	for tri in corners["metal"]:
		for i in 3:
			for other in corners["wood"]:
				for j in 3:
					var p: Vector3 = tri[0][i] + node.position
					var q: Vector3 = other[0][j] + node.position
					if absf(p.x - q.x) < 1e-4 and absf(p.y - q.y) < 1e-4:
						var d: Vector2 = tri[1][i] - other[1][j]
						shifts.append(Vector2(snappedf(fposmod(d.x, 1.0), 0.001), snappedf(fposmod(d.y, 1.0), 0.001)))
	var offset_ok := not shifts.is_empty()
	for d: Vector2 in shifts:
		offset_ok = offset_ok and [0.25, 0.75].has(d.x) and [0.125, 0.875].has(d.y)
	if not offset_ok:
		problems.append("metal offset shifts %s, want a quarter and an eighth of a tile" % [shifts])
	check(problems.is_empty(), "Textures and UVs",
		"7 textures with source pixels, repeat, upright V, spans, 45 and 90 degree turns, 16 8 offset" if problems.is_empty() else "; ".join(problems))


## Every surface of a painted station: its image as Godot imported it, its UV range.
func paint_surfaces(targetname: String) -> Array:
	var instance := mesh_of(station(targetname))
	var out := []
	for index in instance.mesh.get_surface_count():
		var material := instance.get_active_material(index) as StandardMaterial3D
		var lo := Vector2(INF, INF)
		var hi := Vector2(-INF, -INF)
		for tri in triangles(instance.mesh, index):
			for uv: Vector2 in tri[1]:
				lo = lo.min(uv)
				hi = hi.max(uv)
		out.append({"material": material, "image": image_of(material.albedo_texture),
			"source": source_image(material.albedo_texture.resource_path), "lo": lo, "hi": hi})
	return out


func check_colours() -> void:
	var problems := []
	var found := []
	var blended := 0
	var wood := source_image("res://textures/wood.png")
	for surface in paint_surfaces("fl_colours"):
		var material: StandardMaterial3D = surface.material
		var image: Image = surface.image
		if not same_pixels(image, surface.source):
			problems.append("%s: imported pixels differ from the PNG" % material.resource_name)
		if material.texture_repeat:
			problems.append("%s repeats; paint must clamp" % material.resource_name)
		if surface.lo.x < -1e-4 or surface.lo.y < -1e-4 or surface.hi.x > 1.0001 or surface.hi.y > 1.0001:
			problems.append("%s: UVs %s..%s leave the image" % [material.resource_name, surface.lo, surface.hi])
		if material.transparency != BaseMaterial3D.TRANSPARENCY_DISABLED:
			problems.append("%s is transparent" % material.resource_name)
		var c := image.get_pixel(image.get_width() / 2, image.get_height() / 2)
		found.append(c)
		# The translucent face is wood with half the paint over it.
		if image.get_size() == wood.get_size():
			var paint := Color8(30, 200, 220)
			var worst := 0.0
			for y in image.get_height():
				for x in image.get_width():
					var want := wood.get_pixel(x, y).lerp(paint, 128.0 / 255.0)
					var got := image.get_pixel(x, y)
					worst = maxf(worst, maxf(absf(want.r - got.r), maxf(absf(want.g - got.g), absf(want.b - got.b))))
			if worst <= 4.0 / 255.0:
				blended += 1
	for colour in OPAQUE_COLOURS:
		if not found.any(func(c): return colour_near(c, colour, 2.0 / 255.0)):
			problems.append("no face coloured %s" % colour.to_html(false))
	if blended != 1:
		problems.append("%d faces are wood under the 128/255 paint, want 1" % blended)
	check(problems.is_empty(), "Face colours",
		"5 opaque colours, translucent paint over wood within 4/255, clamped, opaque" if problems.is_empty() else "; ".join(problems))


func check_gradients() -> void:
	var problems := []
	var surfaces := paint_surfaces("fl_gradients")
	for stops in GRADIENT_STOPS:
		var hit := false
		for surface in surfaces:
			# The radial centre falls between texels of an even-sized image.
			if stops.all(func(c): return image_has(surface.image, c, 12.0 / 255.0)):
				hit = true
		if not hit:
			problems.append("no image holds stops %s" % [stops.map(func(c): return c.to_html(false))])
	for surface in surfaces:
		if not same_pixels(surface.image, surface.source):
			problems.append("%s: imported pixels differ from the PNG" % surface.material.resource_name)
		if surface.material.texture_repeat:
			problems.append("%s repeats" % surface.material.resource_name)
	check(problems.is_empty(), "Gradients",
		"two-colour, three-stop, radial and black-to-white images imported exactly, clamped" if problems.is_empty() else "; ".join(problems))


func check_surfaces() -> void:
	var instance := mesh_of(station("fl_surface"))
	var found := []
	var problems := []
	for index in instance.mesh.get_surface_count():
		var material := instance.get_active_material(index) as StandardMaterial3D
		var emission := material.emission_energy_multiplier if material.emission_enabled else 0.0
		found.append(Vector3(material.roughness, material.metallic, emission))
		# The editor lights an emissive face with its own colour times the strength.
		if material.emission_enabled and (material.emission_texture != material.albedo_texture
				or material.emission_operator != BaseMaterial3D.EMISSION_OP_MULTIPLY or material.emission != Color.WHITE):
			problems.append("emission is %s %s the face's image, not the image itself" % [material.emission,
				"times" if material.emission_operator == BaseMaterial3D.EMISSION_OP_MULTIPLY else "plus"])
	for want in SURFACES:
		if not found.any(func(v): return v.is_equal_approx(want)):
			problems.append("no surface with roughness, metallic, emission %s" % want)
	check(problems.is_empty(), "Surface materials",
		"roughness, metallic and emission %s" % [found] if problems.is_empty() else "; ".join(problems))


func check_normal_map() -> void:
	var instance := mesh_of(station("fl_normal"))
	var material := instance.get_active_material(0) as StandardMaterial3D
	var normal := material.normal_texture
	var ok := material.normal_enabled and normal != null and normal.get_size() == Vector2(512, 512)
	var arrays := instance.mesh.surface_get_arrays(0)
	check(ok and same_pixels(image_of(normal), source_image("res://textures/brick_wall_normal.png")), "Brush normal maps",
		"brick_wall carries the pixels of textures/brick_wall_normal.png (%s), 512 x 512, normal_enabled; tangents %s"
			% [normal.resource_path if normal else "none", arrays[Mesh.ARRAY_TANGENT] != null])


func check_hidden_and_skip() -> void:
	var hidden := station("fl_hidden")
	var drawn := sides(triangles(mesh_of(hidden).mesh, 0))
	var shape := (hidden.get_node("shape") as CollisionShape3D).shape as ConcavePolygonShape3D
	var faces := shape.get_faces()
	var collided := []
	for i in range(0, faces.size(), 3):
		var n := (faces[i + 2] - faces[i]).cross(faces[i + 1] - faces[i]).normalized()
		var axis := Vector3(roundf(n.x), roundf(n.y), roundf(n.z))
		if not collided.has(axis):
			collided.append(axis)
	# The hidden face is the map's +X side, Godot's +Z.
	check(drawn.size() == 5 and not drawn.has(Vector3.BACK) and collided.size() == 6, "Export-hidden faces",
		"render mesh faces %s, collision faces %d" % [drawn, collided.size()])
	var skip := station("fl_skip")
	var skip_drawn := sides(triangles(mesh_of(skip).mesh, 0))
	check(skip_drawn.size() == 5 and not skip_drawn.has(Vector3.UP), "Skip faces",
		"render mesh faces %s (no top)" % [skip_drawn])


func ray(from: Vector3, to: Vector3) -> Dictionary:
	var space := level.get_world_3d().direct_space_state
	var query := PhysicsRayQueryParameters3D.create(from, to)
	return space.intersect_ray(query)


func check_collision() -> void:
	var solid := station("fl_solid")
	var top := ray(solid.position + Vector3(0, 5, 0), solid.position - Vector3(0, 5, 0))
	check(not top.is_empty() and top.collider == solid and absf(top.position.y - (solid.position.y + 1.0)) < 0.01,
		"Solid collision", "a ray from above stops on fl_solid's top at %s" % top.get("position"))
	var floor := ray(Vector3(-5, 3, 0), Vector3(-5, -3, 0))
	check(not floor.is_empty() and floor.collider.name == "worldspawn" and absf(floor.position.y) < 0.01,
		"Solid collision", "the worldspawn floor stops a ray at %s" % floor.get("position"))
	var clip := station("fl_clip")
	var clip_hit := ray(clip.position + Vector3(0, 5, 0), clip.position - Vector3(0, 5, 0))
	var visuals := clip.find_children("*", "VisualInstance3D", true, false)
	check(not clip_hit.is_empty() and clip_hit.collider == clip and absf(clip_hit.position.y - (clip.position.y + 1.5)) < 0.01 and visuals.is_empty(),
		"Clip brushes", "solid top at %s, %d visual nodes" % [clip_hit.get("position"), visuals.size()])
	var skip := station("fl_skip")
	var down := ray(skip.position + Vector3(0, 5, 0), skip.position - Vector3(0, 5, 0))
	var side := ray(skip.position + Vector3(5, 0, 0), skip.position)
	check((down.is_empty() or down.collider != skip) and not side.is_empty() and side.collider == skip,
		"Skip faces", "a ray down passes the skip top (hit %s), a ray from the side stops on fl_skip" % [down.get("collider", null)])
	var hidden := station("fl_hidden")
	var through := ray(hidden.position + Vector3(0, 0, 5), hidden.position)
	check(not through.is_empty() and through.collider == hidden, "Export-hidden faces",
		"a ray at the hidden side still stops on its collision at %s" % through.get("position"))


## The editor's pose (keyframe_animation.rs, animation_curves.rs) of the mover at
## `time`: X linear 0 128 0 at 0 2 4, Z held 0 32 0 at 0 1 3, RZ bezier 0 to 90 with
## flat handles, all in map units and degrees.
func mover_pose(time: float) -> Array:
	var t := fposmod(time, 4.0)
	var x := 64.0 * t if t <= 2.0 else 128.0 - 64.0 * (t - 2.0)
	var z := 32.0 if t >= 1.0 and t < 3.0 else 0.0
	var s := t / 4.0
	var rz := 90.0 * (3.0 * s * s - 2.0 * s * s * s)
	return [Vector3(x, 0, z), rz]


## Where the editor puts a map point of an animated entity: turn about the pivot by
## X, then Y, then Z, then add the offset.
func editor_point(p: Vector3, pivot: Vector3, offset: Vector3, rz: float) -> Vector3:
	var v := p - pivot
	var r := deg_to_rad(rz)
	v = Vector3(cos(r) * v.x - sin(r) * v.y, sin(r) * v.x + cos(r) * v.y, v.z)
	return v + pivot + offset


func pivot_of(node: Node3D) -> Vector3:
	var parts: PackedStringArray = node.get_meta("properties")["animation_pivot"].split(" ")
	return Vector3(float(parts[0]), float(parts[1]), float(parts[2]))


## Worst distance between the mesh corners as Godot has them and as the editor would.
func pose_error(node: Node3D, rest: Transform3D, offset: Vector3, rz: float) -> float:
	var instance := mesh_of(node)
	var pivot := pivot_of(node)
	var worst := 0.0
	for p in instance.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]:
		var at_rest := to_map(rest * p)
		var want := to_godot(editor_point(at_rest, pivot, offset, rz))
		worst = maxf(worst, want.distance_to(node.global_transform * p))
	return worst


## AnimatableBody3D syncs to physics: a transform set by the animation shows on the
## node after the next physics step.
func check_animation() -> void:
	var mover := station("fl_mover")
	var driver := mover.get_node("HeronAnimation")
	var player := driver.get_node("AnimationPlayer") as AnimationPlayer
	var animation := player.get_animation("default")
	check(player.is_playing() and player.current_animation == "default" and animation.loop_mode == Animation.LOOP_LINEAR and animation.length == 4.0,
		"Keyframe animation", "fl_mover plays `default` on start, looping over %s s" % animation.length)
	var rest: Transform3D = driver.rest_transform
	player.pause()
	var worst := 0.0
	var samples := []
	for t in [0.0, 0.5, 0.999, 1.0, 1.5, 2.0, 2.5, 2.999, 3.0, 3.5, 4.0]:
		driver.seek(t)
		await physics_frame
		await physics_frame
		var pose := mover_pose(t if t < 4.0 else 0.0)
		var error := pose_error(mover, rest, pose[0], pose[1])
		worst = maxf(worst, error)
		samples.append("%s:%.4f" % [t, error])
	check(worst < 0.002, "Keyframe animation",
		"seeked poses match the editor's linear X, held Z and bezier turn, worst corner %.4f m over %s" % [worst, samples])
	player.play()


func check_playback() -> void:
	var mover := station("fl_mover")
	var driver := mover.get_node("HeronAnimation")
	var player := driver.get_node("AnimationPlayer") as AnimationPlayer
	var rest: Transform3D = driver.rest_transform
	var worst := 0.0
	var seen := []
	var wrapped := false
	var last := player.current_animation_position
	var started := Time.get_ticks_msec()
	while Time.get_ticks_msec() - started < 5000:
		await physics_frame
		var t := player.current_animation_position
		if t < last:
			wrapped = true
		# The player's clock lands on 2.99999999999999 where a key sits at 3, and
		# which side of the key that is says nothing about the export.
		if absf(t - roundf(t)) < 1e-6 or absf(last - roundf(last)) < 1e-6:
			last = t
			continue
		# The body shows the pose of this step or, synced through physics, the last.
		var now := mover_pose(t)
		var before := mover_pose(last)
		worst = maxf(worst, minf(pose_error(mover, rest, now[0], now[1]), pose_error(mover, rest, before[0], before[1])))
		last = t
		seen.append(t)
	check(wrapped and worst < 0.002 and player.is_playing(), "Keyframe animation",
		"over 5 s of play (%d physics frames) the mover loops and stays within %.4f m of the editor's pose at the player's time, at most one physics step late" % [seen.size(), worst])


func door_error(door: Node3D, rz: float) -> float:
	return pose_error(door, door.get_node("HeronAnimation").rest_transform, Vector3.ZERO, rz)


func walk_into(body: CharacterBody3D, where: Vector3) -> void:
	body.global_position = Vector3(0, 1, 20)
	for i in 3:
		await physics_frame
	body.global_position = where
	for i in 3:
		await physics_frame


func wait(seconds: float) -> void:
	var started := Time.get_ticks_msec()
	while Time.get_ticks_msec() - started < seconds * 1000.0:
		await physics_frame


func check_trigger_and_button() -> void:
	var door := station("fl_door")
	var trigger := station("fl_trigger") as Area3D
	var button := station("fl_button") as Area3D
	var player := door.get_node("HeronAnimation/AnimationPlayer") as AnimationPlayer
	check(not player.is_playing() and door_error(door, 0.0) < 0.001, "Door, button and trigger links",
		"fl_door waits closed until something opens it")
	var body := CharacterBody3D.new()
	var shape := CollisionShape3D.new()
	var capsule := CapsuleShape3D.new()
	capsule.radius = 0.3
	capsule.height = 1.0
	shape.shape = capsule
	body.add_child(shape)
	level.add_child(body)
	var entered := []
	trigger.body_entered.connect(func(b): entered.append(["trigger", b]))
	button.body_entered.connect(func(b): entered.append(["button", b]))
	await walk_into(body, trigger.global_position)
	check(entered.has(["trigger", body]), "Trigger volumes", "trigger_multiple's Area3D reports body_entered for a CharacterBody3D")
	check(player.current_animation == "open", "Door, button and trigger links", "the trigger starts the door's `open` clip")
	await wait(1.3)
	var opened := door_error(door, 90.0)
	check(opened < 0.002, "Door, button and trigger links", "after the open clip the door is turned 90 degrees about its pivot (%.4f m off)" % opened)
	await walk_into(body, button.global_position)
	check(entered.has(["button", body]) and player.current_animation == "close", "Door, button and trigger links",
		"the button's Area3D fires and toggles the open door to `close`")
	await wait(1.3)
	check(door_error(door, 0.0) < 0.002, "Door, button and trigger links", "after the close clip the door is back at rest")
	await walk_into(body, button.global_position)
	await wait(1.3)
	check(door_error(door, 90.0) < 0.002, "Door, button and trigger links", "the button toggles it open again")
	body.queue_free()


func check_water() -> void:
	var water := station("fl_water")
	var volume := water.get_node("heron_water")
	var instance := mesh_of(water)
	var material := instance.get_active_material(0) as ShaderMaterial
	var normals_up := true
	for tri in triangles(instance.mesh, 0):
		normals_up = normals_up and tri[2].y > 0.99
	check(material != null and material.shader != null and material.shader.code.contains("shader_type spatial") and normals_up,
		"Water", "the wave ShaderMaterial is on a mesh of upward faces only")
	var depth: float = volume.sample(water.global_position)
	var outside: float = volume.sample(water.global_position + Vector3(0, 3, 0))
	check(absf(depth - 0.75) < 0.001 and outside < 0.0 and float(volume.data.swim_speed) == 2.5, "Water",
		"the volume script samples depth %.3f at the centre, %.1f above, swim_speed %s" % [depth, outside, volume.data.swim_speed])
	var view_script: GDScript = load("res://feature_lab/heron_water_view.gd")
	var camera := Camera3D.new()
	level.add_child(camera)
	camera.global_position = water.global_position
	camera.current = true
	var view := Node.new()
	view.set_script(view_script)
	camera.add_child(view)
	await process_frame
	await process_frame
	var fogged := camera.environment != null and camera.environment.fog_enabled and is_equal_approx(camera.environment.fog_density, 0.18)
	camera.global_position = Vector3(0, 3, 20)
	await process_frame
	await process_frame
	check(view_script != null and view_script.can_instantiate() and fogged and camera.environment == null, "Water",
		"heron_water_view.gd loads, fogs a camera in the water at density 0.18 and gives the old environment back")
	camera.queue_free()


func check_sky() -> void:
	var environment: Environment = (level.get_node("HeronEnvironment") as WorldEnvironment).environment
	var material := environment.sky.sky_material as PanoramaSkyMaterial if environment.sky != null else null
	var panorama: Texture2D = material.panorama if material != null else null
	check(environment.background_mode == Environment.BG_SKY and panorama != null and panorama.get_width() > 0,
		"Sky and fog", "panorama sky %s %s" % [panorama.resource_path if panorama else "missing", panorama.get_size() if panorama else Vector2.ZERO])
	check(environment.fog_enabled and is_equal_approx(environment.fog_density, 0.02) and colour_near(environment.fog_light_color, Color8(180, 190, 200), 0.5 / 255.0),
		"Sky and fog", "fog density %s colour %s" % [environment.fog_density, environment.fog_light_color])


## The editor's spot direction (scene_lights.rs `facing`): pitch down positive, yaw about +Z.
func facing(pitch: float, yaw: float) -> Vector3:
	var p := deg_to_rad(pitch)
	var y := deg_to_rad(yaw)
	return Vector3(cos(p) * cos(y), cos(p) * sin(y), -sin(p))


func check_lights() -> void:
	var omni := station("fl_light") as OmniLight3D
	check(omni != null and is_equal_approx(omni.omni_range, 8.0) and is_equal_approx(omni.light_energy, 2.5) and colour_near(omni.light_color, Color8(255, 200, 120), 0.5 / 255.0)
		and omni.global_position.is_equal_approx(to_godot(Vector3(-160, -288, 160))),
		"Lights", "OmniLight3D at %s, range %s, energy %s" % [omni.global_position, omni.omni_range, omni.light_energy])
	var spot := station("fl_spot") as SpotLight3D
	var direction := -spot.global_transform.basis.z.normalized()
	var want := to_godot(facing(-60, 45)).normalized()
	check(spot != null and is_equal_approx(spot.spot_range, 10.0) and is_equal_approx(spot.spot_angle, 30.0) and direction.distance_to(want) < 0.001,
		"Lights", "SpotLight3D range %s, angle %s, shines along %s as the editor's angles -60 45 0 say (%s)" % [spot.spot_range, spot.spot_angle, direction, want])


func check_models() -> void:
	var a := station("fl_model_a")
	var b := station("fl_model_b")
	var mesh_a := mesh_of(a)
	var mesh_b := mesh_of(b)
	var scale_ratio := b.global_transform.basis.get_scale().x / a.global_transform.basis.get_scale().x
	var turn := rad_to_deg(b.global_transform.basis.get_euler().y)
	check(mesh_a.mesh != null and mesh_a.mesh == mesh_b.mesh and absf(scale_ratio - 1.5) < 1e-4 and absf(turn - 45.0) < 0.01,
		"Imported models, shared", "both copies hold one ArrayMesh %s, scale ratio %.3f, turn %.1f degrees" % [mesh_a.mesh.resource_path, scale_ratio, turn])
	var material := mesh_a.get_active_material(0) as StandardMaterial3D
	check(material.normal_enabled and material.normal_texture != null and material.roughness_texture != null and material.ao_enabled and material.ao_texture != null and material.emission_texture != null,
		"Model PBR maps", "normal, metallic-roughness, occlusion and emissive maps load")


func check_properties() -> void:
	var probe: Dictionary = station("fl_probe").get_meta("properties")
	var typed: bool = typeof(probe.charge) == TYPE_INT and probe.charge == 7 and typeof(probe.armed) == TYPE_BOOL and probe.armed \
		and typeof(probe.sequence) == TYPE_ARRAY and probe.sequence == [2, 3, 5] and probe.label == "probe" and probe.note == "free text, with a comma"
	var light: Dictionary = station("fl_light").get_meta("properties")
	var camera: Dictionary = station("fl_camera").get_meta("properties")
	var water: Dictionary = station("fl_water").get_meta("properties")
	typed = typed and typeof(light.light_energy) == TYPE_FLOAT and typeof(light.light_color) == TYPE_COLOR \
		and typeof(camera.fov) == TYPE_FLOAT and typeof(camera.current) == TYPE_INT \
		and typeof(water.swim_speed) == TYPE_FLOAT and typeof(water.water_priority) == TYPE_INT
	check(typed, "Typed properties", "fl_probe %s; light, camera and water values keep float, int and Color" % probe)
	var links := 0
	var problems := []
	for node in level.get_children():
		var properties: Dictionary = node.get_meta("properties", {})
		for path in node.get_meta("target_paths", []):
			var target := node.get_node_or_null(path)
			if target == null or target.get_meta("properties").get("targetname") != properties.get("target"):
				problems.append("%s -> %s" % [node.name, path])
			else:
				links += 1
	check(links == 4 and problems.is_empty(), "Target links", "%d target paths resolve to the named entity %s" % [links, problems])


## Two pictures per station with the scene's own light and sky: as exported, and
## `-nofog` without the map's fog, which the editor's view does not draw.
func take_shots() -> void:
	DirAccess.make_dir_recursive_absolute(shots)
	var camera := Camera3D.new()
	level.add_child(camera)
	camera.current = true
	camera.fov = 60
	var views := {
		"overview": [Vector3(-16, 14, 14), Vector3(2, 0, -14)],
		"textures_a": [Vector3(-4.5, 3.2, -24), Vector3(-7.5, 1.25, -27)],
		"textures_b": [Vector3(-10.5, 3.2, -30), Vector3(-7.5, 1.25, -27)],
		"colours_a": [Vector3(-5.5, 3.5, -17.5), Vector3(-9, 1.25, -21)],
		"colours_b": [Vector3(-12.5, 3.5, -24.5), Vector3(-9, 1.25, -21)],
		"gradients_a": [Vector3(-5.5, 3.5, -11.5), Vector3(-9, 1.25, -15)],
		"gradients_b": [Vector3(-12.5, 3.5, -18.5), Vector3(-9, 1.25, -15)],
		"surface_a": [Vector3(-5.5, 3.0, -5.5), Vector3(-9, 1.25, -9)],
		"surface_b": [Vector3(-12.5, 3.0, -12.5), Vector3(-9, 1.25, -9)],
		"normal": [Vector3(-6, 2.0, 0), Vector3(-9, 1.25, -3)],
		"solid_clip_skip": [Vector3(6, 6, -12), Vector3(1, 1.25, -18)],
		"hidden": [Vector3(4, 3, -5), Vector3(1, 1.25, -9)],
		"mover_door": [Vector3(5, 4, -17), Vector3(11, 1, -22)],
		"water": [Vector3(8, 3, -6), Vector3(12, 1, -10)],
		"models": [Vector3(2.5, 2.5, 6), Vector3(5, 1.2, 6)],
	}
	var environment: Environment = (level.get_node("HeronEnvironment") as WorldEnvironment).environment
	for fog in [true, false]:
		environment.fog_enabled = fog
		for name in views:
			camera.global_position = views[name][0]
			camera.look_at(views[name][1])
			for i in 8:
				await process_frame
			await RenderingServer.frame_post_draw
			var image := root.get_texture().get_image()
			var file: String = name + ("" if fog else "-nofog") + ".png"
			var error := image.save_png(shots.path_join(file))
			print("SHOT %s %s %s" % [file, error, image.get_size()])
