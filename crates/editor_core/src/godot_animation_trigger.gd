extends Node
## Per-trigger state, shared by all of this trigger's animation targets.
@export var targets: Array = []
@export var once: bool = false
@export var action: String = "toggle"
@export var clip: String = ""
@export var delay: float = 0.0
var _used: bool = false
var _last: float = -INF
func _on_body_entered(body: Node3D) -> void:
	if not body is CharacterBody3D:
		return
	var now := Time.get_ticks_msec() * 0.001
	if (once and _used) or now - _last < delay:
		return
	var delivered := false
	for path in targets:
		var target := get_node_or_null(path)
		if target != null:
			match action:
				"play": delivered = target.play(clip) or delivered
				"open", "close": delivered = target.play(action) or delivered
				"pause": target.pause(); delivered = true
				"resume": target.resume(); delivered = true
				"stop": target.stop(); delivered = true
				"", "toggle": target._on_triggered(body); delivered = true
	if delivered:
		_used = true
		_last = now
