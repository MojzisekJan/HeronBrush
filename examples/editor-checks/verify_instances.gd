extends SceneTree

# Run against the Godot fixture emitted by repeated_models_share_assets.
func _initialize() -> void:
    call_deferred("verify")

func verify() -> void:
    var packed = load("res://instances.tscn") as PackedScene
    if packed == null:
        push_error("Could not load exported scene")
        quit(1)
        return
    var scene = packed.instantiate()
    root.add_child(scene)
    var actual: Array[Vector3] = []
    var meshes = scene.find_children("*", "MeshInstance3D", true, false)
    var normals: Array[Vector3] = []
    var resources: Dictionary = {}
    for instance in meshes:
        resources[instance.mesh.get_instance_id()] = true
        for surface in instance.mesh.get_surface_count():
            var arrays = instance.mesh.surface_get_arrays(surface)
            for vertex in arrays[Mesh.ARRAY_VERTEX]:
                actual.append(instance.global_transform * vertex)
            for normal in arrays[Mesh.ARRAY_NORMAL]:
                normals.append((instance.global_transform.basis.inverse().transposed() * normal).normalized())
    var expected = JSON.parse_string(FileAccess.get_file_as_string("res://expected.json"))
    var errors: Array[String] = []
    if meshes.size() != 3 or resources.size() != 1:
        errors.append("Expected three instances sharing one mesh; got %d / %d" % [meshes.size(), resources.size()])
    if actual.size() != expected.vertices.size():
        errors.append("Unexpected vertex count")
    for point in expected.vertices:
        var wanted = Vector3(point[0], point[1], point[2])
        var found = false
        for vertex in actual:
            if vertex.distance_to(wanted) < 0.0001:
                found = true
                break
        if not found:
            errors.append("Missing world vertex %s" % wanted)
    for normal in expected.normals:
        var wanted = Vector3(normal[0], normal[1], normal[2])
        var found = false
        for actual_normal in normals:
            if actual_normal.distance_to(wanted) < 0.0001:
                found = true
                break
        if not found:
            errors.append("Missing transformed normal %s" % wanted)
    if not errors.is_empty():
        for error in errors:
            push_error(error)
        quit(1)
        return
    print("PASS: three native Godot instances share one mesh and preserve world vertices")
    scene.queue_free()
    quit()
