extends SceneTree
func _initialize() -> void:
    call_deferred("verify")
func materials_of(scene: Node) -> Array:
    var result: Array = []
    for node in scene.find_children("*", "MeshInstance3D", true, false):
        for index in node.mesh.get_surface_count(): result.append(node.get_active_material(index))
    return result
func verify() -> void:
    var source = load("res://source.glb").instantiate()
    var exported = load("res://modes.tscn").instantiate()
    var reference = materials_of(source)
    var actual = materials_of(exported)
    var errors: Array[String] = []
    if reference.size() != 6 or actual.size() != 6:
        errors.append("Expected six source and six exported materials")
    else:
        for index in 6:
            var a = actual[index]
            var r = reference[index]
            for property in ["transparency", "cull_mode"]:
                if a.get(property) != r.get(property): errors.append(r.resource_name + " changed " + property)
            if r.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR:
                if abs(a.alpha_scissor_threshold - r.alpha_scissor_threshold) > 0.0001:
                    errors.append(r.resource_name + " changed cutoff")
            var ai = a.albedo_texture.get_image()
            var ri = r.albedo_texture.get_image()
            if ai.get_size() != ri.get_size(): errors.append(r.resource_name + " changed albedo dimensions")
            else:
                for y in ai.get_height():
                    for x in ai.get_width():
                        var aa = ai.get_pixel(x, y).a * a.albedo_color.a
                        var ra = ri.get_pixel(x, y).a * r.albedo_color.a
                        if abs(aa - ra) > 1.0 / 255.0:
                            errors.append(r.resource_name + " changed effective alpha")
    source.free()
    exported.free()
    if errors.is_empty():
        print("PASS: exported Godot material modes, cutoff, culling and effective alpha match native source")
        quit()
    else:
        for error in errors: push_error(error)
        quit(1)
