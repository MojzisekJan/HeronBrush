extends SceneTree
var lab: Node3D
var errors: Array[String] = []
var measurements := {}
func _initialize() -> void:
	call_deferred("check")
func frames(count: int) -> void:
	for i in count:
		await physics_frame
		await process_frame
func expect(condition: bool, message: String) -> void:
	if not condition:
		errors.append(message)
func reset_all() -> void:
	lab.player.position = Vector3(16,1,12)
	lab.player.velocity = Vector3.ZERO
	for name in ["shuttle","lift","rotor","door"]:
		lab.animated(name).get_node("HeronAnimation").stop()
	await frames(8)
func ride(name: String, seconds: float, offset: Vector3) -> void:
	await reset_all()
	var body: Node3D = lab.animated(name)
	var animation = body.get_node("HeronAnimation")
	lab.player.position = body.global_position + offset
	lab.player.velocity = Vector3.ZERO
	await frames(30)
	expect(lab.player.is_on_floor(),name+": player did not land")
	var anchor: Vector3 = body.to_local(lab.player.global_position)
	animation.play()
	var max_error := 0.0
	var airborne := 0
	var max_vertical_error := 0.0
	var lo: Vector3 = body.position
	var hi: Vector3 = body.position
	var turned := false
	var left_path := false
	for i in int(seconds*60):
		await frames(1)
		lo = lo.min(body.position)
		hi = hi.max(body.position)
		turned = turned or absf(body.rotation.y) > 1.0
		if name == "shuttle" and body.position.x < -4.04 and body.position.z > -5.96:
			left_path = true
		var actual: Vector3 = body.to_local(lab.player.global_position)
		max_error = maxf(max_error,Vector2(actual.x-anchor.x,actual.z-anchor.z).length())
		max_vertical_error = maxf(max_vertical_error,absf(actual.y-anchor.y))
		if not lab.player.is_on_floor():airborne += 1
	measurements[name] = {"horizontal_drift":max_error,"vertical_drift":max_vertical_error,"airborne_frames":airborne,"seconds":seconds}
	if name == "shuttle":
		expect(hi.x-lo.x > 5.9 and hi.z-lo.z > 5.9,"Shuttle did not traverse both segments")
		expect(not left_path,"Shuttle cut diagonally across its L path")
	if name == "lift":expect(hi.y-lo.y > 5.9,"Lift did not reach both floors")
	if name == "rotor":
		expect(turned,"Rotating platform remained stationary")
		expect(max_error < 0.01,"Idle rotation correction regressed")
	expect(max_error<0.16,name+": horizontal drift %.4f m" % max_error)
	expect(max_vertical_error<0.12,name+": vertical drift %.4f m" % max_vertical_error)
	expect(airborne<=2,name+": lost floor contact for %d frames" % airborne)
	if name == "rotor":
		var before: Vector3 = body.to_local(lab.player.global_position)
		lab.player.desired_move = (body.global_position-lab.player.global_position).normalized()
		lab.player.desired_move.y = 0
		await frames(10)
		lab.player.desired_move = Vector3.ZERO
		expect(body.to_local(lab.player.global_position).distance_to(before)>0.4,"Player cannot walk on rotating platform")
	# Jumping must detach instead of being glued to the moving body.
	lab.player.jump_requested = true
	await frames(12)
	expect(not lab.player.is_on_floor(),name+": jump failed to detach")
func check() -> void:
	lab = load("res://main.tscn").instantiate()
	lab.testing = true
	root.add_child(lab)
	await frames(5)
	for name in ["shuttle","lift","rotor","door"]:
		expect(lab.animated(name) is AnimatableBody3D,"Missing body "+name)
	await ride("shuttle",17.0,Vector3(0,0.4,0))
	await ride("lift",15.0,Vector3(0,0.4,0))
	await ride("rotor",120.0,Vector3(1,0.4,0))
	await reset_all()
	var door = lab.animated("door")
	var animation = door.get_node("HeronAnimation")
	# Test the actual exported Area3D signal by walking onto its pad.
	lab.player.position = Vector3(8,0.02,3)
	await frames(35)
	expect(animation.playing,"Door pad did not start animation")
	expect(absf(door.rotation.y)>0.05,"Door did not rotate")
	await frames(100)
	expect(absf(door.rotation.y-PI/2)<0.02,"Door did not fully open")
	# Open passage is clear; closed door blocks a capsule moving through it.
	lab.player.position = Vector3(8,0.1,4.5)
	lab.player.velocity = Vector3.ZERO
	lab.player.desired_move = Vector3.BACK
	await frames(35)
	expect(lab.player.position.z>6.5,"Open doorway blocked player")
	lab.player.desired_move = Vector3.ZERO
	animation.stop()
	await frames(5)
	lab.player.position = Vector3(8,0.1,4.5)
	lab.player.velocity = Vector3.ZERO
	lab.player.desired_move = Vector3.BACK
	await frames(35)
	expect(lab.player.position.z<5.8,"Closed door collision failed")
	lab.player.desired_move = Vector3.ZERO
	# Lift starts only through its own exported button.
	var lift = lab.animated("lift").get_node("HeronAnimation")
	lift.stop()
	lab.player.position = Vector3(6,0.02,-3)
	await frames(10)
	expect(lift.playing,"Lift pad did not start animation")
	print(JSON.stringify(measurements))
	var file := FileAccess.open("res://verification.json",FileAccess.WRITE)
	file.store_string(JSON.stringify({"measurements":measurements,"errors":errors},"  ")+"\n")
	if errors.is_empty():
		print("HERON_MOTION_LAB_OK")
		quit(0)
	else:
		for error in errors:push_error(error)
		quit(1)
