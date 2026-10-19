extends SceneTree
# Checks the source fixture's native interpretation, not Heron Brush exports.
func _initialize() -> void:
    call_deferred("verify")
func verify() -> void:
    var scene = load("res://source.glb").instantiate()
    var found: Dictionary = {}
    var errors: Array[String] = []
    for node in scene.find_children("*", "MeshInstance3D", true, false):
        for index in node.mesh.get_surface_count():
            var material = node.get_active_material(index)
            found[material.resource_name] = material
    for mode in ["OPAQUE", "MASK", "BLEND"]:
        for double in [false, true]:
            var name = mode + ("_double" if double else "_single")
            if not found.has(name):
                errors.append("Missing source material " + name)
                continue
            var material = found[name]
            var transparency = BaseMaterial3D.TRANSPARENCY_DISABLED
            if mode == "MASK": transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
            # Godot's glTF importer uses alpha blending with a depth prepass.
            if mode == "BLEND": transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_DEPTH_PRE_PASS
            if material.transparency != transparency: errors.append(name + " transparency")
            var cull = BaseMaterial3D.CULL_DISABLED if double else BaseMaterial3D.CULL_BACK
            if material.cull_mode != cull: errors.append(name + " culling")
            if mode == "MASK" and abs(material.alpha_scissor_threshold - 0.42) > 0.0001:
                errors.append(name + " cutoff")
            if abs(material.albedo_color.a - 0.75) > 0.0001:
                errors.append(name + " alpha factor")
            var image = material.albedo_texture.get_image()
            var alphas: Array[int] = []
            for y in image.get_height():
                for x in image.get_width(): alphas.append(roundi(image.get_pixel(x, y).a * 255.0))
            alphas.sort()
            if alphas != [0, 96, 192, 255]: errors.append(name + " alpha gradient")
    if found.size() != 6: errors.append("Expected exactly six source materials")
    scene.free()
    if errors.is_empty():
        print("PASS: native source OPAQUE/MASK/BLEND, sidedness, cutoff, alpha factor and gradient")
        quit()
    else:
        for error in errors: push_error(error)
        quit(1)
