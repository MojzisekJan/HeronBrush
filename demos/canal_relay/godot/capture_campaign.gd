extends SceneTree
func _initialize() -> void:
	call_deferred("capture")
func capture() -> void:
	var game: Node3D = load("res://main.tscn").instantiate()
	root.add_child(game)
	await process_frame
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	for chapter in game.CHAPTERS:
		game.load_level(chapter)
		game.player.enabled = false
		game.player.global_position = Vector3(13, 13, -15)
		game.player.camera.look_at(Vector3(0, 0, 0))
		for i in range(12): await process_frame
		await RenderingServer.frame_post_draw
		var path := "/tmp/heron-canal-%s.png" % chapter
		root.get_texture().get_image().save_png(path)
		print(path)
	quit()
