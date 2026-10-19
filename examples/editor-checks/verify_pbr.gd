extends SceneTree
func _initialize() -> void:
    call_deferred("verify")
func materials_of(scene: Node) -> Array:
    var materials: Array = []
    for node in scene.find_children("*", "MeshInstance3D", true, false):
        for surface in node.mesh.get_surface_count():
            materials.append(node.get_active_material(surface))
    materials.sort_custom(func(a, b): return a.roughness < b.roughness)
    return materials
func verify() -> void:
    var actual_scene = load("res://pbr.tscn").instantiate()
    var reference_scene = load("res://source.glb").instantiate()
    var actual = materials_of(actual_scene)
    var reference = materials_of(reference_scene)
    var errors: Array[String] = []
    if actual.size() != 2 or reference.size() != 2:
        errors.append("Both PBR materials must load")
    else:
        for i in 2:
            var a = actual[i]
            var r = reference[i]
            print("material ", i, " roughness=", a.roughness, " metallic=", a.metallic, " emission=", a.emission, " reference emission=", r.emission)
            for property in ["roughness", "metallic", "normal_scale"]:
                if abs(a.get(property) - r.get(property)) > 0.0001:
                    errors.append("Changed " + property)
            if not a.normal_enabled or not a.ao_enabled or not a.emission_enabled:
                errors.append("A PBR feature is disabled")
            if not a.emission.is_equal_approx(r.emission):
                errors.append("Emissive factor differs from the native GLB importer")
            if a.normal_texture.get_width() != 2 or a.roughness_texture.get_height() != 1 or a.ao_texture.get_height() != 2:
                errors.append("Map dimensions changed")
            if a.roughness_texture_channel != 1 or a.metallic_texture_channel != 2:
                errors.append("Wrong metallic-roughness channels")
            var red = a.ao_texture.get_image().get_pixel(0, 0).r
            if abs(red - 207.0 / 255.0) > 0.005:
                errors.append("AO strength was not applied to the Godot companion image")
    actual_scene.free()
    reference_scene.free()
    if errors.is_empty():
        print("PASS: Godot PBR maps, factors, dimensions, channels and AO strength")
        quit()
    else:
        for error in errors:
            push_error(error)
        quit(1)
