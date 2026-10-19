extends SceneTree
func _initialize() -> void:
	call_deferred("check")
func fail(message: String) -> void:
	push_error(message)
	quit(1)
func check() -> void:
	var cases = JSON.parse_string(FileAccess.get_file_as_string("res://expected.json"))
	for test in cases:
		var scene = load(test.scene).instantiate()
		root.add_child(scene)
		var player = scene.find_child("HeronAnimation", true, false)
		if player == null:
			fail("Missing animation player")
			return
		var body = player.get_parent()
		if not body is AnimatableBody3D or body.find_child("shape_*",true,false) == null:
			fail("Missing animated collision body")
			return
		if player.playing != test.autoplay:
			fail("Autoplay mismatch")
			return
		player.pause()
		body.sync_to_physics = false
		for sample in test.samples:
			player.seek(sample.time)
			var locals = [Vector3.ZERO,Vector3.BACK,Vector3.RIGHT,Vector3.UP]
			for i in range(4):
				var actual: Vector3 = body.transform * locals[i]
				var p = sample.points[i]
				var expected := Vector3(p[0],p[1],p[2])
				if actual.distance_to(expected) > 0.00003:
					fail("%s time %s point %s: %s != %s" % [test.scene,sample.time,i,actual,expected])
					return
		player.play()
		player.get_node("AnimationPlayer").advance(4.5)
		var last_sample = test.samples[-1]
		for i in range(4):
			var locals = [Vector3.ZERO,Vector3.BACK,Vector3.RIGHT,Vector3.UP]
			var p = last_sample.points[i]
			if (body.transform * locals[i]).distance_to(Vector3(p[0],p[1],p[2])) > 0.00003:
				fail("Native advance/loop mismatch: " + test.scene)
				return
		player.stop()
		if body.position.distance_to(Vector3(3,4,2)) > 0.00003:
			fail("Stop did not restore authored transform")
			return
		body.sync_to_physics = true
		player.play()
		for i in range(6):
			await physics_frame
		player.pause()
		if body.position.distance_to(Vector3(3,4,2)) < 0.00001:
			fail("Physics playback did not move collision body")
			return
		if test.scene.ends_with("case0.tscn"):
			player.seek(1.999)
			player.resume()
			await physics_frame
			await physics_frame
			if player.playing:
				fail("Once playback did not finish")
				return
		scene.queue_free()
		await process_frame
	var scene = load("res://door.tscn").instantiate()
	root.add_child(scene)
	var controller = scene.find_child("HeronAnimation", true, false)
	var native = controller.get_node("AnimationPlayer")
	var button = scene.find_child("func_button",true,false)
	var body = controller.get_parent()
	body.sync_to_physics = false
	var closed: Transform3D = body.transform
	if not native is AnimationPlayer or not native.has_animation("open") or not native.has_animation("close"):
		fail("Missing native named clips")
		return
	var actor := CharacterBody3D.new()
	button.body_entered.emit(actor)
	if native.assigned_animation != "open" or not native.is_playing():
		fail("Button did not start open")
		return
	native.advance(0.3)
	var midway: Transform3D = body.transform
	button.body_entered.emit(actor)
	if native.assigned_animation != "close" or not body.transform.is_equal_approx(midway):
		fail("Reversing a moving door jumped")
		return
	native.advance(1.1)
	if not body.transform.is_equal_approx(closed):
		fail("Door did not return to its closed pose")
		return
	button.body_entered.emit(actor)
	native.advance(1.1)
	if body.transform.is_equal_approx(closed):
		fail("Door did not open fully")
		return
	button.body_entered.emit(actor)
	native.advance(1.1)
	if not body.transform.is_equal_approx(closed):
		fail("Second button press did not close the door")
		return
	var relay = button.get_node("HeronAnimationTrigger")
	relay.once = true
	relay._used = false
	var prop := StaticBody3D.new()
	button.body_entered.emit(prop)
	if relay._used:
		fail("Non-player consumed a once trigger")
		return
	button.body_entered.emit(actor)
	button.body_entered.emit(actor)
	if native.assigned_animation != "open":
		fail("Once trigger fired twice")
		return
	relay.once = false
	relay.delay = 0.0
	relay.action = "play"
	relay.clip = "close"
	button.body_entered.emit(actor)
	if native.assigned_animation != "close": fail("Named clip action failed"); return
	native.advance(0.25)
	var paused: Transform3D = body.transform
	relay.action = "pause"
	button.body_entered.emit(actor)
	if native.is_playing() or not body.transform.is_equal_approx(paused): fail("Pause action failed"); return
	relay.action = "resume"
	button.body_entered.emit(actor)
	if not native.is_playing(): fail("Resume action failed"); return
	relay.action = "stop"
	button.body_entered.emit(actor)
	if native.is_playing() or not body.transform.is_equal_approx(closed): fail("Stop action failed"); return
	relay.action = "open"
	button.body_entered.emit(actor)
	if native.assigned_animation != "open": fail("Open action failed"); return
	relay.action = "close"
	button.body_entered.emit(actor)
	if native.assigned_animation != "close": fail("Close action failed"); return
	relay.delay = 10.0
	relay.action = "open"
	button.body_entered.emit(actor)
	if native.assigned_animation != "close": fail("Repeat cooldown failed"); return
	prop.free()
	actor.free()
	scene.queue_free()
	await process_frame
	print("Native AnimationPlayer open/close, button connections and reversal passed")
	print("Godot animation: 5 exported scenes, 45 poses, 180 transformed points and physics playback passed")
	quit(0)
