extends SceneTree

var frames := 0
var out_path := "res://screenshot.png"

func _initialize() -> void:
	root.size = Vector2i(1280, 720)
	var packed: PackedScene = load("res://maps/example.tscn")
	var scene: Node = packed.instantiate()
	root.add_child(scene)

	var env := WorldEnvironment.new()
	var e := Environment.new()
	e.background_mode = Environment.BG_COLOR
	e.background_color = Color(0.05, 0.06, 0.08)
	e.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	e.ambient_light_color = Color(0.35, 0.38, 0.45)
	e.ambient_light_energy = 0.6
	e.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	env.environment = e
	root.add_child(env)

	var cam := Camera3D.new()
	root.add_child(cam)
	cam.fov = 70
	cam.look_at_from_position(Vector3(-7.0, 4.5, -7.0), Vector3(1.0, 1.0, 1.5), Vector3.UP)
	cam.current = true

func _process(_delta: float) -> bool:
	frames += 1
	if frames < 20:
		return false
	var image := root.get_texture().get_image()
	image.save_png(out_path)
	print("wrote ", out_path, " ", image.get_width(), "x", image.get_height())
	return true
