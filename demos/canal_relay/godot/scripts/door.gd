extends AnimatableBody3D
var opened := false
func open_gate(immediate := false) -> void:
	if opened:
		return
	opened = true
	var props: Dictionary = get_meta("properties", {})
	var rise := float(props.get("rise", 3.0))
	if immediate:
		position.y += rise
	else:
		var tween := create_tween().set_process_mode(Tween.TWEEN_PROCESS_PHYSICS)
		tween.tween_property(self, "position:y", position.y + rise, 0.8)
