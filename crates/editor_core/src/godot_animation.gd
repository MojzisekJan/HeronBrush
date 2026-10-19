@tool
extends Node
## Native AnimationPlayer supplies scalar channels; this adapter applies the map pivot.
signal finished
@export var default_clip: String = "default"
@export var autoplay: bool = true
@export var pivot: Vector3 = Vector3.ZERO
@export var map_scale: float = 0.03125
@export var rest_transform: Transform3D = Transform3D.IDENTITY
var _opened: bool = false
var _player: AnimationPlayer
var playing: bool:
	get: return is_instance_valid(_player) and _player.is_playing()
@export var map_x: float = 0.0:
	set(value):
		map_x = value
		_apply()
@export var map_y: float = 0.0:
	set(value):
		map_y = value
		_apply()
@export var map_z: float = 0.0:
	set(value):
		map_z = value
		_apply()
@export var map_rx: float = 0.0:
	set(value):
		map_rx = value
		_apply()
@export var map_ry: float = 0.0:
	set(value):
		map_ry = value
		_apply()
@export var map_rz: float = 0.0:
	set(value):
		map_rz = value
		_apply()
@export var step_x: float = 0.0:
	set(value):
		step_x = value
		_apply()
@export var step_y: float = 0.0:
	set(value):
		step_y = value
		_apply()
@export var step_z: float = 0.0:
	set(value):
		step_z = value
		_apply()
@export var step_rx: float = 0.0:
	set(value):
		step_rx = value
		_apply()
@export var step_ry: float = 0.0:
	set(value):
		step_ry = value
		_apply()
@export var step_rz: float = 0.0:
	set(value):
		step_rz = value
		_apply()

func _ready() -> void:
	_player = get_node("AnimationPlayer")
	_player.animation_finished.connect(_finished)
	if autoplay and not Engine.is_editor_hint():
		play()

func _apply() -> void:
	if not is_inside_tree():
		return
	var body := get_parent() as Node3D
	if body == null:
		return
	var rotation := Basis(Vector3.UP, deg_to_rad(map_rz + step_rz)) * Basis(Vector3.RIGHT, deg_to_rad(map_ry + step_ry)) * Basis(Vector3.BACK, deg_to_rad(map_rx + step_rx))
	var offset := Vector3(map_y + step_y, map_z + step_z, map_x + step_x) * map_scale
	body.transform = Transform3D(rotation * rest_transform.basis, rotation * (rest_transform.origin - pivot) + pivot + offset)

func play(clip: String = "") -> bool:
	var name := default_clip if clip.is_empty() else clip
	if not _player.has_animation(name):
		push_warning("Unknown animation: " + name)
		return false
	_player.stop()
	_player.play(name)
	_player.advance(0.0)
	if name == "open": _opened = true
	elif name == "close": _opened = false
	return true

func pause() -> void:
	_player.pause()

func resume() -> void:
	_player.play()

func stop() -> void:
	_player.stop()
	get_parent().transform = rest_transform
	_opened = false

func seek(seconds: float) -> void:
	if _player.assigned_animation.is_empty():
		_player.assigned_animation = default_clip
	var animation := _player.get_animation(_player.assigned_animation)
	var duration: float = animation.length
	var t: float = maxf(seconds, 0.0)
	if animation.loop_mode == Animation.LOOP_LINEAR:
		t = fposmod(t, duration)
	elif animation.loop_mode == Animation.LOOP_PINGPONG:
		t = fposmod(t, duration * 2.0)
		if t > duration:
			t = duration * 2.0 - t
	else:
		t = minf(t, duration)
	_player.seek(t, true)

func _finished(_name: StringName) -> void:
	finished.emit()

func _on_triggered(body: Node3D = null) -> void:
	if body != null and not body is CharacterBody3D:
		return
	if _player.has_animation("open") and _player.has_animation("close"):
		var phase := 0.0
		var reverse := _player.is_playing()
		if reverse:
			phase = _player.current_animation_position / _player.current_animation_length
		_opened = not _opened
		play("open" if _opened else "close")
		if reverse:
			_player.seek((1.0 - phase) * _player.current_animation_length, true)
	else:
		play()
