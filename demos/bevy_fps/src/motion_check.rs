//! Native motion_lab regression using exported GLB collision and Avian.
use super::tests::{app_before_finish, tick};
use super::*;
use crate::heron_animation::{
    HeronActivator, HeronAnimation, HeronAnimationFixedPlugin, HeronAnimationName,
    HeronAnimationSet, HeronAnimationTrigger,
};
use bevy::ecs::system::RunSystemOnce;
use std::{collections::HashMap, path::Path};

fn pose(app: &App, entity: Entity) -> Transform {
    *app.world().get::<Transform>(entity).unwrap()
}
fn feet(app: &App, player: Entity) -> Vec3 {
    app.world().get::<Position>(player).unwrap().0 - Vec3::Y
}
fn teleport(app: &mut App, player: Entity, feet: Vec3) {
    let position = feet + Vec3::Y;
    app.world_mut().entity_mut(player).insert((
        Position(position),
        Transform::from_translation(position),
        LinearVelocity::ZERO,
    ));
}
fn stop(app: &mut App, roots: &HashMap<String, Entity>, player: Entity) {
    for &entity in roots.values() {
        if let Some(mut animation) = app.world_mut().get_mut::<HeronAnimation>(entity) {
            animation.stop();
        }
    }
    teleport(app, player, Vec3::new(16.0, 1.0, 12.0));
    tick(app, 8);
}

fn exported_colliders(app: &mut App, directory: &Path) -> HashMap<String, Entity> {
    exported_colliders_for(app,directory,"motion_lab.json")
}

fn exported_colliders_for(app:&mut App,directory:&Path,name:&str)->HashMap<String,Entity>{
    let level = level::load(&directory.join(name)).unwrap();
    let scope=app.world_mut().spawn_empty().id();
    let mut roots = HashMap::new();
    for source in &level.entities {
        let root = app
            .world_mut()
            .spawn((
                source.transform.to_transform(),
                crate::heron_entities::HeronEntityData::from_json(&serde_json::to_value(source).unwrap()).unwrap(),
                crate::heron_entities::HeronEntityScope(scope),
                body_for(source),
                HeronAnimationName(source.name.clone()),
            ))
            .id();
        if let Some(json) = source
            .properties
            .get("_heron_animation")
            .and_then(level::Value::as_str)
        {
            app.world_mut()
                .entity_mut(root)
                .insert(HeronAnimation::from_json(json).unwrap());
        }
        if let Some(json) = source
            .properties
            .get("_heron_trigger")
            .and_then(level::Value::as_str)
        {
            app.world_mut()
                .entity_mut(root)
                .insert(HeronAnimationTrigger::from_json(json).unwrap());
        }
        roots.insert(source.name.clone(), root);
        let (Some(collision), Some(mesh)) = (&source.collision, &source.mesh) else {
            continue;
        };
        let bytes = std::fs::read(directory.join(mesh)).unwrap();
        let glb = gltf::Gltf::from_slice(&bytes).unwrap();
        let mesh = glb.meshes().nth(collision.mesh).unwrap();
        assert_eq!(mesh.primitives().count(), collision.parts);
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|_| glb.blob.as_deref());
            let points: Vec<_> = reader.read_positions().unwrap().collect();
            let mut geometry = Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points);
            if let Some(indices) = reader.read_indices() {
                geometry.insert_indices(bevy::mesh::Indices::U32(indices.into_u32().collect()));
            }
            let collider = match collision.kind {
                CollisionShape::Convex => convex_from_points(&geometry),
                CollisionShape::Trimesh => trimesh_from_triangles(&geometry),
            }
            .unwrap();
            let child = app
                .world_mut()
                .spawn((
                    collider,
                    Transform::default(),
                    ChildOf(root),
                    Friction::ZERO,
                    Restitution::ZERO,
                ))
                .id();
            if collision.sensor {
                app.world_mut()
                    .entity_mut(child)
                    .insert((Sensor, CollidingEntities::default()));
            }
        }
    }
    roots
}

#[derive(Component)]
struct PuzzleState {charge:i64,armed:bool,target:Entity}
fn puzzle_factory(world:&mut World,entity:Entity,data:&crate::heron_entities::HeronEntityData,names:&HashMap<String,Entity>)->Result<(),String>{
    assert_eq!(data.properties["armed"],true);
    assert_eq!(data.properties["sequence"],serde_json::json!([2,3,5]));
    let target=names[&data.targets[0]];
    world.entity_mut(entity).insert(PuzzleState{charge:0,armed:true,target});
    Ok(())
}
fn beacon_factory(world:&mut World,entity:Entity,data:&crate::heron_entities::HeronEntityData,_:&HashMap<String,Entity>)->Result<(),String>{
    assert_eq!(data.properties["charge"],7);
    world.entity_mut(entity).insert(PuzzleState{charge:7,armed:false,target:entity});
    Ok(())
}

#[test]
#[ignore="export examples/engine-checks/bevy/prepare_custom_game.py first"]
fn exported_custom_game_factories_physics_and_multiple_levels(){
    use crate::heron_entities::{configure_entities,HeronEntityFactories,HeronEntityConfigured};
    let directory=std::path::PathBuf::from("/tmp/heron-bevy-custom-game/export");
    let mut app=app_before_finish();
    app.insert_resource(Gravity(Vec3::NEG_Y*9.81));app.finish();app.cleanup();
    let first=exported_colliders_for(&mut app,&directory,"workshop.json");
    let second=exported_colliders_for(&mut app,&directory,"workshop.json");
    app.insert_resource(HeronEntityFactories(HashMap::from([
        ("panel".into(),puzzle_factory as crate::heron_entities::HeronEntityFactory),
        ("beacon".into(),beacon_factory as crate::heron_entities::HeronEntityFactory),
    ])));
    configure_entities(app.world_mut());
    for roots in [&first,&second]{
        let panel=*roots.iter().find(|(_,entity)|app.world().get::<PuzzleState>(**entity).is_some_and(|state|state.armed)).unwrap().1;
        let target=app.world().get::<PuzzleState>(panel).unwrap().target;
        assert_eq!(target,roots["beacon"]);
        assert!(app.world().get::<HeronEntityConfigured>(panel).is_some());
        app.world_mut().get_mut::<PuzzleState>(target).unwrap().charge+=1;
        assert_eq!(app.world().get::<PuzzleState>(target).unwrap().charge,8);
    }
    configure_entities(app.world_mut());
    assert_eq!(app.world().get::<PuzzleState>(first["beacon"]).unwrap().charge,8,"factory ran twice");
    // Despawn the second copy before physics: overlapping copies must not collide.
    for entity in second.values(){app.world_mut().despawn(*entity);}
    let panel=*first.iter().find(|(_,entity)|app.world().get::<PuzzleState>(**entity).is_some_and(|state|state.armed)).unwrap().1;
    assert_eq!(app.world().get::<RigidBody>(panel),Some(&RigidBody::Dynamic));
    tick(&mut app,240);
    let height=app.world().get::<Position>(panel).unwrap().y;
    assert!((height-0.5).abs()<0.08,"custom dynamic convex body did not settle: {height}");
    println!("HERON_BEVY_CUSTOM_GAME_OK: native ECS factories, typed properties, scoped references, dynamic GLB hull, reconfiguration guard");
}

#[test]
#[ignore = "export motion_lab with examples/engine-checks/export_motion_lab.py bevy first"]
fn exported_motion_lab_rides_jumps_and_triggers() {
    let directory = std::env::var_os("HERON_BEVY_MOTION")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/tmp/heron-motion-bevy".into());
    let mut app = app_before_finish();
    app.insert_resource(Gravity(Vec3::NEG_Y * 22.0))
        .init_resource::<PlatformPoses>()
        .add_plugins(HeronAnimationFixedPlugin)
        .add_systems(FixedPreUpdate, carry_player.after(HeronAnimationSet));
    app.finish();
    app.cleanup();
    let roots = exported_colliders(&mut app, &directory);
    let player = app
        .world_mut()
        .spawn((
            Player,
            RigidBody::Dynamic,
            Collider::capsule(0.3, 1.4),
            Transform::from_xyz(16.0, 2.0, 12.0),
            LockedAxes::ROTATION_LOCKED,
            Friction::ZERO,
            Restitution::ZERO,
            LinearVelocity::ZERO,
        ))
        .id();
    app.world_mut().spawn((
        HeronActivator,
        Transform::from_xyz(0.0, -1.0, 0.0),
        ChildOf(player),
    ));
    tick(&mut app, 5);
    for (name, seconds, offset) in [
        ("func_animate", 17, Vec3::new(0.0, 0.4, 0.0)),
        ("func_animate_1", 15, Vec3::new(0.0, 0.4, 0.0)),
        ("func_animate_2", 120, Vec3::new(1.0, 0.4, 0.0)),
    ] {
        stop(&mut app, &roots, player);
        let platform = roots[name];
        let target = pose(&app, platform).translation + offset;
        teleport(&mut app, player, target);
        tick(&mut app, 60);
        let anchor = pose(&app, platform)
            .compute_affine()
            .inverse()
            .transform_point3(feet(&app, player));
        app.world_mut()
            .get_mut::<HeronAnimation>(platform)
            .unwrap()
            .resume();
        let (mut drift, mut vertical) = (0.0_f32, 0.0_f32);
        let mut lo = pose(&app, platform).translation;
        let mut hi = lo;
        let (mut turned, mut cut) = (false, false);
        let mut airborne = 0;
        let excluded: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<Sensor>>()
            .iter(app.world())
            .chain([player])
            .collect();
        for _ in 0..seconds * 60 {
            tick(&mut app, 1);
            let transform = pose(&app, platform);
            let local = transform
                .compute_affine()
                .inverse()
                .transform_point3(feet(&app, player));
            drift = drift.max(Vec2::new(local.x - anchor.x, local.z - anchor.z).length());
            vertical = vertical.max((local.y - anchor.y).abs());
            lo = lo.min(transform.translation);
            hi = hi.max(transform.translation);
            turned |= transform.rotation.y.abs() > 0.4;
            cut |= name == "func_animate"
                && transform.translation.x < -4.04
                && transform.translation.z > -5.96;
            let position = app.world().get::<Position>(player).unwrap().0;
            let filter = SpatialQueryFilter::from_excluded_entities(excluded.iter().copied());
            let mut hit = app
                .world_mut()
                .run_system_once(move |spatial: SpatialQuery| {
                    spatial
                        .cast_ray(position, Dir3::NEG_Y, 1.08, true, &filter)
                        .map(|hit| hit.entity)
                })
                .unwrap();
            while let Some(entity) = hit {
                if entity == platform {
                    break;
                }
                hit = app.world().get::<ChildOf>(entity).map(ChildOf::parent);
            }
            if hit != Some(platform) {
                airborne += 1;
            }
        }
        println!(
            "{name}: drift={drift:.5}, vertical={vertical:.5}, airborne={airborne}, travel={:?}",
            hi - lo
        );
        assert!(
            drift < 0.16 && vertical < 0.12,
            "{name}: player lost platform"
        );
        assert!(!cut, "shuttle cut across L path");
        assert!(
            airborne <= 2,
            "{name}: lost floor contact for {airborne} frames"
        );
        if name == "func_animate" {
            assert!((hi - lo).x > 5.9 && (hi - lo).z > 5.9);
        }
        if name == "func_animate_1" {
            assert!((hi - lo).y > 5.9);
        }
        if name == "func_animate_2" {
            assert!(turned, "rotor stayed stationary");
            assert!(drift < 0.01, "rotor idle rotation correction regressed");
            let anchor = pose(&app, platform)
                .compute_affine()
                .inverse()
                .transform_point3(feet(&app, player));
            for _ in 0..12 {
                let direction = (pose(&app, platform).translation - feet(&app, player))
                    .with_y(0.0)
                    .normalize();
                let mut velocity = app.world_mut().get_mut::<LinearVelocity>(player).unwrap();
                velocity.x = direction.x * 4.0;
                velocity.z = direction.z * 4.0;
                drop(velocity);
                tick(&mut app, 1);
            }
            let local = pose(&app, platform)
                .compute_affine()
                .inverse()
                .transform_point3(feet(&app, player));
            assert!(
                (local - anchor).length() > 0.4,
                "player cannot walk on rotor"
            );
            app.world_mut().get_mut::<LinearVelocity>(player).unwrap().x = 0.0;
            app.world_mut().get_mut::<LinearVelocity>(player).unwrap().z = 0.0;
        }
        let before = feet(&app, player);
        app.world_mut().get_mut::<LinearVelocity>(player).unwrap().y = 7.0;
        tick(&mut app, 12);
        assert!(
            feet(&app, player).y > before.y + 0.4,
            "{name}: jump did not detach"
        );
    }
    stop(&mut app, &roots, player);
    let door = roots["func_animate_3"];
    teleport(&mut app, player, Vec3::new(8.0, 0.02, 3.0));
    tick(&mut app, 35);
    assert!(
        app.world()
            .get::<HeronAnimation>(door)
            .unwrap()
            .is_playing(),
        "door pad failed"
    );
    tick(&mut app, 100);
    let yaw = pose(&app, door).rotation.to_euler(EulerRot::YXZ).0;
    assert!(
        (yaw.abs() - std::f32::consts::FRAC_PI_2).abs() < 0.02,
        "door did not open"
    );
    teleport(&mut app, player, Vec3::new(8.0, 0.1, 4.5));
    for _ in 0..35 {
        app.world_mut().get_mut::<LinearVelocity>(player).unwrap().z = 4.0;
        tick(&mut app, 1);
    }
    let open_z = feet(&app, player).z;
    assert!(open_z > 6.5, "open door blocks player at {open_z}");
    app.world_mut()
        .get_mut::<HeronAnimation>(door)
        .unwrap()
        .stop();
    tick(&mut app, 5);
    teleport(&mut app, player, Vec3::new(8.0, 0.1, 4.5));
    for _ in 0..35 {
        app.world_mut().get_mut::<LinearVelocity>(player).unwrap().z = 4.0;
        tick(&mut app, 1);
    }
    let closed_z = feet(&app, player).z;
    assert!(
        closed_z < 5.8,
        "closed door fails to block player at {closed_z}"
    );
    println!("door: open z={open_z}, closed z={closed_z}");
    let lift = roots["func_animate_1"];
    app.world_mut()
        .get_mut::<HeronAnimation>(lift)
        .unwrap()
        .stop();
    tick(&mut app, 5);
    teleport(&mut app, player, Vec3::new(6.0, 0.02, -3.0));
    tick(&mut app, 10);
    assert!(
        app.world()
            .get::<HeronAnimation>(lift)
            .unwrap()
            .is_playing(),
        "lift pad failed"
    );
    println!("HERON_MOTION_LAB_BEVY_OK");
}
