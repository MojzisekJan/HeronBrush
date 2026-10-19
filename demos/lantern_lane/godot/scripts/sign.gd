extends Area3D
## A volume that says something when the player walks into it: a street name, a
## word about the church, a nudge about what to do next.

var title := ""
var text := ""
var seconds := 3.0
var once := true
var said := false


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	title = str(props.get("title", ""))
	text = str(props.get("text", ""))
	seconds = float(props.get("seconds", 3.0))
	once = int(props.get("once", 1)) != 0
	body_entered.connect(_on_entered)


func _on_entered(body: Node3D) -> void:
	if not body.is_in_group("player") or (once and said):
		return
	said = true
	var level: Node = _level()
	if level != null and level.has_method("say"):
		level.say(_line(), seconds)


func _line() -> String:
	if title.is_empty():
		return text
	if text.is_empty():
		return title
	return "%s\n%s" % [title, text]


## The map is instanced into the scene that runs it, so `owner` is the map's own
## root rather than the level script. Walking up finds whoever is listening.
func _level() -> Node:
	var at: Node = get_parent()
	while at != null:
		if at.has_method("say"):
			return at
		at = at.get_parent()
	return null
