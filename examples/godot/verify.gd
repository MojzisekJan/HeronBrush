extends SceneTree

var failures := 0

func check(ok: bool, label: String) -> void:
	if ok:
		print("  PASS  ", label)
	else:
		print("  FAIL  ", label)
		failures += 1

func _initialize() -> void:
	print("=== scene ===")
	var packed := load("res://maps/example.tscn") as PackedScene
	check(packed != null, "example.tscn loads")
	var root := packed.instantiate()
	check(root != null, "scene instantiates")
	print("  root: ", root.name, " (", root.get_class(), ") with ", root.get_child_count(), " children")

	for child in root.get_children():
		var extra := ""
		if child is Node3D:
			extra = " at %v" % child.global_position if child.is_inside_tree() else " pos %v" % child.position
		print("    ", child.name, " : ", child.get_class(), extra)

	var world := root.get_node_or_null("worldspawn")
	check(world is StaticBody3D, "worldspawn is a StaticBody3D")
	check(world.get_node_or_null("shape") is CollisionShape3D, "worldspawn has a collision shape")
	check(world.get_node("shape").shape is ConcavePolygonShape3D, "collision is concave")

	var light := root.get_node_or_null("light")
	check(light is OmniLight3D, "light is an OmniLight3D")
	check(is_equal_approx(light.light_energy, 4.0), "light_energy is 4.0, got %s" % light.light_energy)
	check(is_equal_approx(light.omni_range, 12.0), "omni_range is 12.0, got %s" % light.omni_range)
	check(light.light_color.is_equal_approx(Color(1.0, 240.0/255.0, 214.0/255.0, 1.0)), "light_color survived")

	check(root.get_node_or_null("light_spot") is SpotLight3D, "light_spot is a SpotLight3D")
	check(root.get_node_or_null("light_sun") is DirectionalLight3D, "light_sun is a DirectionalLight3D")
	check(root.get_node_or_null("func_door") is AnimatableBody3D, "func_door is an AnimatableBody3D")
	check(root.get_node_or_null("trigger_multiple") is Area3D, "trigger_multiple is an Area3D")
	check(root.get_node("trigger_multiple").collision_mask == 2, "collision_mask is 2")
	check(root.get_node_or_null("func_physics") is RigidBody3D, "func_physics is a RigidBody3D")
	check(is_equal_approx(root.get_node("func_physics").mass, 25.0), "mass is 25.0")
	var sound := root.get_node_or_null("ambient_sound") as AudioStreamPlayer3D
	check(sound != null and sound.stream != null, "ambient_sound loaded its AudioStream")
	check(root.get_node_or_null("layer_7_Detail") != null, "the map layer became a node")
	check(root.get_node_or_null("layer_7_Detail/func_illusionary") != null, "the layer holds its entity")

	check(root.get_node_or_null("prop_static") != null, "prop_static instanced its scene")
	var props: Dictionary = root.get_node("func_door").get_meta("properties")
	check(props.get("targetname") == "door1", "metadata carries targetname")
	check(is_equal_approx(props.get("wait", 0.0), 3.0), "metadata filled in the FGD default")

	var start := root.get_node_or_null("info_player_start")
	check(start.position.is_equal_approx(Vector3(-6, 1, 0)), "player start at %v" % start.position)

	print("=== mesh ===")
	var mesh := load("res://maps/example/worldspawn.obj") as ArrayMesh
	check(mesh != null, "worldspawn.obj loads as an ArrayMesh")
	print("  surfaces: ", mesh.get_surface_count())
	for i in mesh.get_surface_count():
		print("    ", i, ": ", mesh.surface_get_name(i))

	var mi := world.get_node_or_null("mesh") as MeshInstance3D
	check(mi != null and mi.mesh != null, "MeshInstance3D has its mesh")
	check(mi.get_surface_override_material(0) != null, "surface 0 has a material override")
	var mat := mi.get_surface_override_material(0) as StandardMaterial3D
	check(mat != null and mat.albedo_texture != null, "the material carries an albedo texture")

	# Winding: Godot treats a triangle as front facing when its vertices run
	# clockwise as seen from the front, so the cross product points away from
	# the shading normal.
	var backwards := 0
	var total := 0
	for s in mesh.get_surface_count():
		var arrays := mesh.surface_get_arrays(s)
		var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var norms: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
		var idx: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
		for t in range(0, idx.size(), 3):
			var a := verts[idx[t]]
			var b := verts[idx[t + 1]]
			var c := verts[idx[t + 2]]
			var geometric := (b - a).cross(c - a)
			if geometric.length() < 0.000001:
				continue
			total += 1
			if geometric.normalized().dot(norms[idx[t]]) > 0.0:
				backwards += 1
	print("  triangles: ", total, ", wound against their normal: ", backwards)
	check(backwards == 0, "every triangle faces outward")

	# UVs: the floor brush corner at map (-256 -256 0) uses a 64 pixel texture
	# at scale 1, so its texture coordinate must be exactly (-4, 4).
	var found := false
	for s in mesh.get_surface_count():
		if mesh.surface_get_name(s) != "floor":
			continue
		var arrays := mesh.surface_get_arrays(s)
		var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var norms: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
		var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
		for i in verts.size():
			# The importer stores normals octahedrally, so compare by angle
			# rather than component by component.
			if verts[i].is_equal_approx(Vector3(-8, 0, -8)) and norms[i].dot(Vector3.UP) > 0.999:
				found = true
				print("  floor corner uv: ", uvs[i], " expected (-4, 4)")
				check(uvs[i].is_equal_approx(Vector2(-4, 4)), "the floor corner uv is right side up")
				break
	check(found, "found the floor corner vertex")

	print("")
	if failures == 0:
		print("ALL CHECKS PASSED")
	else:
		print(failures, " CHECKS FAILED")
	quit(failures)
