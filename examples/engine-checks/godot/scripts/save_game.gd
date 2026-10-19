class_name SaveGame
extends RefCounted
## Where a game in progress is kept between runs: which level, and what the
## player carries into it. Everything else comes back as the level was built.

const PATH := "user://save.cfg"
const SECTION := "game"


static func exists() -> bool:
	return FileAccess.file_exists(PATH)


static func write(state: Dictionary) -> void:
	var config := ConfigFile.new()
	for key in state:
		config.set_value(SECTION, str(key), state[key])
	config.set_value(SECTION, "saved_at", Time.get_datetime_string_from_system())
	config.save(PATH)


static func read() -> Dictionary:
	var config := ConfigFile.new()
	if config.load(PATH) != OK:
		return {}
	var out := {}
	for key in config.get_section_keys(SECTION):
		out[key] = config.get_value(SECTION, key)
	return out


static func clear() -> void:
	if exists():
		DirAccess.remove_absolute(ProjectSettings.globalize_path(PATH))
