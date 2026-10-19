@tool
extends EditorPlugin

const MANIFEST = "res://.heron-brush-live.json"
const ACK = "res://.heron-brush-live-status.json"
var panel: VBoxContainer
var status: Label
var paused: CheckButton
var elapsed := 0.0
var busy := false
var latest: Dictionary = {}
var applied := ""
var attached: Node

func _enter_tree() -> void:
    panel = VBoxContainer.new()
    panel.name = "Heron Brush"
    status = Label.new()
    status.text = "Waiting for Heron Brush"
    status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    panel.add_child(status)
    var attach := Button.new()
    attach.text = "Attach map to current scene"
    attach.pressed.connect(_attach)
    panel.add_child(attach)
    paused = CheckButton.new()
    paused.text = "Pause updates"
    panel.add_child(paused)
    var hint := Label.new()
    hint.text = "Only the linked map branch is replaced. Put your own Godot nodes outside it. Save the scene in Godot to keep changes."
    hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    panel.add_child(hint)
    add_control_to_dock(DOCK_SLOT_RIGHT_BL, panel)
    set_process(true)

func _exit_tree() -> void:
    set_process(false)
    remove_control_from_docks(panel)
    panel.queue_free()

func _process(delta: float) -> void:
    elapsed += delta
    if elapsed < 0.25 or busy or paused.button_pressed:
        return
    elapsed = 0.0
    if FileAccess.file_exists("res://.heron-brush-live-busy"):
        return
    if not FileAccess.file_exists(MANIFEST):
        status.text = "Live link stopped"
        return
    var parsed = JSON.parse_string(FileAccess.get_file_as_string(MANIFEST))
    if not parsed is Dictionary or parsed.get("version") != 1:
        return
    latest = parsed
    if str(latest.get("revision", "")) == applied:
        return
    var root := EditorInterface.get_edited_scene_root()
    if root == null:
        status.text = "Create or open a 3D scene, then attach the map"
        _ack("waiting", status.text)
        return
    attached = null
    for child in root.get_children():
        if child.get_meta("heron_brush_source", "") == latest.get("source", ""):
            attached = child
            break
    if not is_instance_valid(attached):
        status.text = "Map ready — attach it to the current scene"
        _ack("waiting", status.text)
        return
    _update(false)

func _attach() -> void:
    if busy or latest.is_empty():
        return
    var root := EditorInterface.get_edited_scene_root()
    if not root is Node3D:
        status.text = "Open a scene with a Node3D root first"
        return
    attached = null
    for child in root.get_children():
        if child.get_meta("heron_brush_source", "") == latest.get("source", ""):
            attached = child
            break
    _update(true)

func _update(allow_attach: bool) -> void:
    var fs := EditorInterface.get_resource_filesystem()
    if fs.is_scanning() or fs.is_importing():
        return
    var root := EditorInterface.get_edited_scene_root()
    if root == null:
        return
    busy = true
    var snapshot := latest.duplicate(true)
    status.text = "Updating map…"
    # Godot normally notices external edits on focus. Explicit scanning and reimporting
    # also work while the mapper keeps drawing in Heron Brush.
    fs.scan()
    while fs.is_scanning() or fs.is_importing():
        await get_tree().process_frame
        if not is_inside_tree():
            return
    var meshes := PackedStringArray()
    for path in snapshot.get("meshes", []):
        if str(path).begins_with("res://") and FileAccess.file_exists(path):
            meshes.append(path)
    if not meshes.is_empty():
        fs.reimport_files(meshes)
    if EditorInterface.get_edited_scene_root() != root:
        busy = false
        return
    var path: String = snapshot.get("scene", "")
    if not path.begins_with("res://") or not FileAccess.file_exists(path):
        status.text = "Exported scene is missing"
        _ack("error", status.text)
        busy = false
        return
    var packed = ResourceLoader.load(path, "PackedScene", ResourceLoader.CACHE_MODE_REPLACE_DEEP)
    if not packed is PackedScene:
        status.text = "Could not load exported scene"
        _ack("error", status.text)
        busy = false
        return
    var next: Node = packed.instantiate(PackedScene.GEN_EDIT_STATE_INSTANCE)
    if paused.button_pressed or not FileAccess.file_exists(MANIFEST):
        next.free()
        busy = false
        return
    var current = JSON.parse_string(FileAccess.get_file_as_string(MANIFEST))
    if FileAccess.file_exists("res://.heron-brush-live-busy") or not current is Dictionary or current.get("revision") != snapshot.get("revision"):
        next.free()
        busy = false
        return
    if not is_instance_valid(attached) and not allow_attach:
        next.free()
        busy = false
        return
    var transform := Transform3D.IDENTITY
    var slot := root.get_child_count()
    if is_instance_valid(attached):
        if attached is Node3D:
            transform = attached.transform
        slot = attached.get_index()
        var selection := EditorInterface.get_selection()
        for selected in selection.get_selected_nodes():
            if selected == attached or attached.is_ancestor_of(selected):
                selection.remove_node(selected)
        root.remove_child(attached)
        attached.queue_free()
    next.name = "HeronBrushMap"
    next.set_meta("heron_brush_source", snapshot.get("source", ""))
    root.add_child(next, true)
    root.move_child(next, mini(slot, root.get_child_count() - 1))
    next.owner = root
    if next is Node3D:
        next.transform = transform
    attached = next
    applied = str(snapshot.get("revision", ""))
    EditorInterface.mark_scene_as_unsaved()
    status.text = "Connected — map updated"
    _ack("updated", status.text, applied)
    busy = false

func _ack(state: String, message: String, revision: String = "") -> void:
    var file := FileAccess.open(ACK, FileAccess.WRITE)
    if file:
        file.store_string(JSON.stringify({"state": state, "message": message, "revision": revision, "source": latest.get("source", "")}))
