extends SceneTree
func _initialize() -> void:
	call_deferred("capture")
func capture() -> void:
	var lab = load("res://main.tscn").instantiate()
	lab.testing = true
	root.add_child(lab)
	var camera := Camera3D.new()
	lab.add_child(camera)
	camera.position = Vector3(24,25,30)
	camera.look_at(Vector3(0,1,0))
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 33
	camera.current = true
	for i in 8:await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	image.save_png("res://../preview.png")
	quit()
