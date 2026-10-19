extends SceneTree
var failures := 0
func check(ok: bool, message: String) -> void:
 print(('PASS ' if ok else 'FAIL ') + message)
 if not ok: failures += 1
func named(root: Node, wanted: String) -> Node:
 for n in root.find_children('*', '', true, false):
  if str(n.get_meta('properties', {}).get('targetname', '')) == wanted: return n
 return null
func _initialize() -> void:
 run.call_deferred()
func run() -> void:
 var main = load('res://scenes/play.tscn').instantiate()
 root.add_child(main)
 for i in range(8): await physics_frame
 var player = main.get_node('Player')
 var a = named(main, 'portal_square')
 var b = named(main, 'portal_terrace')
 check(a != null and b != null, 'both portals exist')
 check(a.destination == b and b.destination == a, 'portal target paths resolve in both directions')
 a.body_entered.emit(player)
 var expected = b.global_position - b.global_transform.basis.z * b.exit_offset
 check(player.global_position.distance_to(expected) < .01, 'entering portal teleports to its linked destination')
 b.body_entered.emit(player)
 check(player.global_position.distance_to(expected) < .01, 'arrival cooldown prevents bouncing back')
 var door = named(main, 'door_-410')
 var door_at = door.position
 var lift = named(main, 'quay_lift')
 var lift_at = lift.position
 var shuttle = named(main, 'cargo_shuttle')
 var shuttle_at = shuttle.position
 var boat = named(main, 'moored_sailboat')
 var boat_at = boat.position
 var spinner = named(main, 'workshop_sign')
 var rotation_at = spinner.rotation
 named(main, 'bakery_sensor').body_entered.emit(player)
 named(main, 'lift_sensor').body_entered.emit(player)
 for i in range(24): await physics_frame
 check(door.position.distance_to(door_at) > .05, 'door trigger moves bakery door')
 check(lift.position.distance_to(lift_at) > .05, 'lift trigger raises quay lift')
 check(shuttle.position.distance_to(shuttle_at) > .05, 'cargo shuttle moves')
 check(spinner.rotation.distance_to(rotation_at) > .01, 'workshop sign rotates')
 check(boat.position.y > boat_at.y and abs(boat.position.x-boat_at.x)<.001 and abs(boat.position.z-boat_at.z)<.001, 'sailboat bobs vertically without drifting')
 check(named(main, 'cargo_crate') is RigidBody3D, 'cargo is a real physics body')
 check(named(main, 'river') is Area3D, 'river is a swim volume, not a solid surface')
 check(named(main, 'canal_water').playing, 'positional ambience plays')
 check(named(main, 'square_checkpoint') is Area3D, 'checkpoint is a sensor')
 check(named(main, 'amber_key') != null and named(main, 'market_gate') != null, 'key and collectible gate are present')
 main.queue_free()
 for i in range(4): await process_frame
 print('Showcase failures: ', failures)
 quit(1 if failures else 0)
