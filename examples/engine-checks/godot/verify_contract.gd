extends SceneTree
var failures := 0
func check(ok: bool, label: String) -> void:
 print(("PASS " if ok else "FAIL ") + label)
 if not ok: failures += 1
func _initialize() -> void:
 run.call_deferred()
func run() -> void:
 var packed: PackedScene = load("res://maps/demo.tscn")
 check(packed != null, "exported scene loads")
 var level := packed.instantiate()
 # Inspect serialized data before runtime scripts mutate it.
 var meshes := level.find_children("*", "MeshInstance3D", true, false)
 var shapes := level.find_children("*", "CollisionShape3D", true, false)
 check(meshes.size() > 12, "exported and instanced visual meshes exist: %d" % meshes.size())
 check(shapes.size() > 12, "exported and instanced collision shapes exist: %d" % shapes.size())
 var references := 0
 var entities := 0
 for node in level.get_children():
  var props: Dictionary = node.get_meta("properties", {})
  if props.is_empty(): continue
  entities += 1
  check(props.has("classname"), "%s retains class metadata" % node.name)
  for path in node.get_meta("target_paths", []):
   references += 1
   check(node.get_node_or_null(path) != null, "%s target resolves to %s" % [node.name, path])
 check(entities >= 45, "all map entities retained: %d" % entities)
 check(references >= 8, "cross-entity references retained: %d" % references)
 var door := level.get_node("func_door")
 var properties: Dictionary = door.get_meta("properties")
 check(properties["speed"] is float and properties["speed"] == 180.0, "FGD numeric property retains its type and value")
 check(properties["open_direction"] == "0 0 1", "custom string parameter retained")
 var portal := level.get_node("fps_portal")
 check(portal.get_meta("properties")["color"] is Color, "FGD color retains its type")
 var paints := {}
 for mesh in meshes:
  if mesh.mesh == null: continue
  for index in mesh.mesh.get_surface_count():
   var material = mesh.get_active_material(index)
   if material is StandardMaterial3D and material.albedo_texture != null:
    var path: String = material.albedo_texture.resource_path
    if "demo_paint/" in path:
     paints[path] = true
     check(material.albedo_texture.get_width() > 0, "paint image loads: " + path)
 check(paints.size() == 2, "both editor-painted materials are usable in Godot")
 level.free()
 print("ALL CHECKS PASSED" if failures == 0 else "%d CHECKS FAILED" % failures)
 quit(failures)
