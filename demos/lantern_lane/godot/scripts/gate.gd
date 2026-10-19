extends Node3D
## Fires whatever it points at once the level says a condition is met.
##
## The condition can be a count of pickups, a named key, or a story flag the
## game sets from a script. Anything already triggerable is a valid target, so
## this opens doors, starts lifts, spawns enemies or lights a window without any
## of those knowing why.
##
## The flag is the general way out: a script that follows the story calls
## `level.set_flag("bells_rung")` and every gate waiting on it opens.

var needs_count := 0
var needs_kind := ""
var needs_key := ""
var needs_flag := ""
var message := ""
var opened := false


func _ready() -> void:
	var props: Dictionary = get_meta("properties", {})
	needs_count = int(props.get("needs_count", 0))
	needs_kind = str(props.get("needs_kind", ""))
	needs_key = str(props.get("needs_key", ""))
	needs_flag = str(props.get("needs_flag", ""))
	message = str(props.get("message", ""))


## Asked by the level whenever something it keeps track of changed.
func check(level: Node) -> void:
	if opened or not met(level):
		return
	opened = true
	if not message.is_empty() and level.has_method("say"):
		level.say(message)
	for path in get_meta("target_paths", []):
		var target: Node = get_node_or_null(path)
		if target != null and target.has_method("_on_triggered"):
			target._on_triggered()


func met(level: Node) -> bool:
	if not needs_flag.is_empty() and not level.has_flag(needs_flag):
		return false
	if not needs_key.is_empty() and not level.has_key(needs_key):
		return false
	if needs_count > 0 and level.count_taken(needs_kind) < needs_count:
		return false
	return true
