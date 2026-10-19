extends SceneTree
var main: Node
var frame := 0
func _initialize() -> void:
 main = load("res://scenes/play.tscn").instantiate()
 main.fallback_level = load("res://maps/demo.tscn")
 root.add_child(main)
func _process(_delta: float) -> bool:
 frame += 1
 if frame == 10:
  var camera := Camera3D.new()
  main.add_child(camera)
  camera.position = Vector3(-9, 4.5, -12)
  camera.look_at(Vector3(0, 1, 0))
  camera.current = true
  main.get_node("HUD").visible = false
  main.get_node("Player/Head/Camera/SubViewportContainer").visible = false
 if frame == 60:
  capture.call_deferred()
 return false
func capture() -> void:
 await RenderingServer.frame_post_draw
 var image := root.get_texture().get_image()
 DirAccess.make_dir_recursive_absolute("res://evidence")
 var error := image.save_png("res://evidence/demo.png")
 print("SCREENSHOT ", error, " ", image.get_size())
 main.free()
 quit(error)
