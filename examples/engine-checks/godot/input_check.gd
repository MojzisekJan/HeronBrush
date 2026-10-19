extends SceneTree
## Shows what the game actually receives while you hold keys down. Cheap
## keyboards drop the third or fourth key of some combinations, and W with A
## with Shift with Space is exactly the cluster that trips it. Any box that
## stays empty is a key lost before the game sees it.

var label: Label
var most := 0
var most_combo := ""

const WATCHED := [
	"move_forward", "move_back", "move_left", "move_right",
	"jump", "sprint", "use", "shoot",
]


func _initialize() -> void:
	get_root().size = Vector2i(720, 460)
	get_root().title = "Keyboard check"

	var layer := CanvasLayer.new()
	get_root().add_child(layer)

	var panel := ColorRect.new()
	panel.color = Color(0.09, 0.10, 0.13)
	panel.anchor_right = 1.0
	panel.anchor_bottom = 1.0
	layer.add_child(panel)

	label = Label.new()
	label.position = Vector2(24, 20)
	label.add_theme_font_size_override("font_size", 20)
	layer.add_child(label)


func _process(_delta: float) -> bool:
	var held: Array[String] = []
	for action in WATCHED:
		if InputMap.has_action(action) and Input.is_action_pressed(action):
			held.append(action)

	if held.size() > most:
		most = held.size()
		most_combo = ", ".join(held)

	var lines: Array[String] = [
		"Hold the keys you normally would and watch the boxes.",
		"Any that stay empty are being dropped by the keyboard.",
		"",
	]
	for action in WATCHED:
		var on: bool = InputMap.has_action(action) and Input.is_action_pressed(action)
		lines.append(("  [X]  " if on else "  [ ]  ") + action)
	lines.append("")
	lines.append("held right now:      %d" % held.size())
	lines.append("most seen together:  %d   %s" % [most, most_combo])
	lines.append("")
	lines.append("Escape closes this.")
	label.text = "\n".join(lines)

	return Input.is_key_pressed(KEY_ESCAPE)
