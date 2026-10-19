extends SceneTree
var player: CharacterBody3D
var scene: Node3D
var frames := 0
func _initialize() -> void:
    call_deferred("start")
func start() -> void:
    scene = load("res://maps/workshop.tscn").instantiate()
    root.add_child(scene)
    var sensor = scene.get_node("sensor_panel")
    assert(sensor is Area3D and sensor.is_in_group("interactions"), "Class binding did not configure the sensor")
    assert(sensor.get_meta("target_paths")[0] == NodePath("../beacon"), "Reference did not resolve")
    player = CharacterBody3D.new()
    var shape = CollisionShape3D.new()
    var capsule = CapsuleShape3D.new()
    capsule.height = 1.8
    capsule.radius = 0.3
    shape.shape = capsule
    shape.position.y = 0.9
    player.add_child(shape)
    player.position = Vector3(0, 4, 0)
    root.add_child(player)
func _physics_process(dt: float) -> bool:
    if not is_instance_valid(player): return false
    player.velocity.y -= 10 * dt
    player.move_and_slide()
    frames += 1
    if frames >= 120:
        var beacon = scene.get_node("beacon")
        if beacon.entered != 1 or not player.is_on_floor() or abs(player.position.y) >= 0.1:
            push_error("Custom project failed: entered=%s, player=%s, grounded=%s, overlaps=%s" % [beacon.entered, player.position, player.is_on_floor(), scene.get_node("sensor_panel").get_overlapping_bodies()])
            quit(2)
            return false
        print("HERON_GODOT_CUSTOM_GAME_OK: custom PackedScene, typed defaults, authored override, native signal, target reference and floor")
        quit(0)
    return false
