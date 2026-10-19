extends AudioStreamPlayer3D
func _ready() -> void:
 if stream is AudioStreamWAV:
  stream = stream.duplicate()
  stream.loop_mode = AudioStreamWAV.LOOP_FORWARD
  stream.loop_begin = 0
  stream.loop_end = stream.data.size() / 2
 if autoplay: play()
