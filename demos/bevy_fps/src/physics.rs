//! Optional physics adapter. Geometry and sensor flags come from the export;
//! gameplay remains in the consuming game. Moving roots move their child colliders.
use crate::{
    level::{self, CollisionShape},
    Args,
};
use avian3d::prelude::*;
use bevy::math::Affine3A;
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};

pub struct LevelPhysicsPlugin;
impl Plugin for LevelPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .insert_resource(Time::<Fixed>::from_hz(60.0))
            .insert_resource(Gravity(Vec3::NEG_Y * 22.0))
            .init_resource::<PlayerSpawned>()
            .init_resource::<PhysicsCheck>()
            .init_resource::<PlatformPoses>()
            .add_systems(Update, check_physics)
            .add_systems(
                Update,
                (load_colliders, activate_bodies, spawn_player, controls).chain(),
            )
            .add_systems(
                FixedPreUpdate,
                carry_player.after(crate::heron_animation::HeronAnimationSet),
            )
            .add_systems(
                PostUpdate,
                follow_player.before(bevy::transform::TransformSystems::Propagate),
            );
    }
}

/// Games may override this policy or replace the component before simulation.
/// Dynamic concave triangle meshes are deliberately refused: export convex parts.
pub fn body_for(entity: &level::Entity) -> RigidBody {
    if entity.properties.contains_key("_heron_animation") {
        return RigidBody::Kinematic;
    }
    match entity
        .properties
        .get("physics_body")
        .and_then(level::Value::as_str)
    {
        Some("dynamic")
            if entity
                .collision
                .as_ref()
                .is_some_and(|c| c.kind == CollisionShape::Convex) =>
        {
            RigidBody::Dynamic
        }
        Some("dynamic") => {
            warn!(
                "{}: dynamic body requires convex collision; using static",
                entity.name
            );
            RigidBody::Static
        }
        Some("kinematic") => RigidBody::Kinematic,
        _ => RigidBody::Static,
    }
}

#[derive(Component)]
pub struct PendingCollider {
    pub kind: CollisionShape,
    pub sensor: bool,
}

// Bevy can expand indexed GLB meshes when generating missing flat normals.
// Accept point data directly; Avian's mesh helper requires an index buffer.
fn convex_from_points(mesh: &Mesh) -> Option<Collider> {
    let bevy::mesh::VertexAttributeValues::Float32x3(points) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
    else {
        return None;
    };
    Collider::convex_hull(points.iter().copied().map(Vec3::from).collect())
}

fn trimesh_from_triangles(mesh: &Mesh) -> Option<Collider> {
    if mesh.primitive_topology() != bevy::mesh::PrimitiveTopology::TriangleList {
        return None;
    }
    let bevy::mesh::VertexAttributeValues::Float32x3(points) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
    else {
        return None;
    };
    let indices: Vec<u32> = match mesh.indices() {
        Some(indices) => indices.iter().map(|i| i as u32).collect(),
        None => (0..points.len() as u32).collect(),
    };
    if indices.is_empty()
        || indices.len() % 3 != 0
        || indices.iter().any(|i| *i as usize >= points.len())
    {
        return None;
    }
    Some(Collider::trimesh(
        points.iter().copied().map(Vec3::from).collect(),
        indices
            .chunks_exact(3)
            .map(|i| [i[0], i[1], i[2]])
            .collect(),
    ))
}

fn load_colliders(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    assets: Res<AssetServer>,
    pending: Query<(Entity, &Mesh3d, &PendingCollider)>,
    mut exit: MessageWriter<AppExit>,
) {
    for (entity, mesh, pending) in &pending {
        let Some(mesh_data) = meshes.get(&mesh.0) else {
            if matches!(
                assets.load_state(mesh.0.id()),
                bevy::asset::LoadState::Failed(_)
            ) {
                error!("Collision mesh failed to load: {:?}", mesh.0);
                exit.write(AppExit::error());
            }
            continue;
        };
        let collider = match pending.kind {
            CollisionShape::Trimesh => trimesh_from_triangles(mesh_data),
            CollisionShape::Convex => convex_from_points(mesh_data),
        };
        let Some(collider) = collider else {
            error!("Invalid exported {:?} collision mesh", pending.kind);
            exit.write(AppExit::error());
            continue;
        };
        let mut c = commands.entity(entity);
        c.insert((collider, Friction::new(0.6), Restitution::ZERO))
            .remove::<PendingCollider>();
        if pending.sensor {
            c.insert((Sensor, CollidingEntities::default()));
        }
    }
}

/// Hold imported bodies still until every collision mesh is available.
#[derive(Component)]
pub struct AwaitingWorld;
fn activate_bodies(
    mut commands: Commands,
    pending: Query<(), With<PendingCollider>>,
    bodies: Query<Entity, With<AwaitingWorld>>,
) {
    if !pending.is_empty() {
        return;
    }
    for body in &bodies {
        commands
            .entity(body)
            .remove::<(RigidBodyDisabled, AwaitingWorld)>();
    }
}

#[derive(Resource, Default)]
struct PlayerSpawned(bool);
#[derive(Component)]
struct Player;
#[derive(Component)]
struct Aim {
    yaw: f32,
    pitch: f32,
}

fn spawn_player(
    mut commands: Commands,
    args: Res<Args>,
    level: Res<level::Level>,
    pending: Query<(), With<PendingCollider>>,
    mut spawned: ResMut<PlayerSpawned>,
    colliders: Query<(), With<Collider>>,
    cameras: Query<(Entity, &Transform), With<Camera3d>>,
) {
    if spawned.0 || args.fly || !pending.is_empty() || colliders.is_empty() {
        return;
    }
    let Some(start) = level
        .entities
        .iter()
        .find(|e| e.classname == "info_player_start")
    else {
        return;
    };
    let position = Vec3::from(start.transform.translation) + Vec3::Y * 1.05;
    commands
        .spawn((
            Name::new("Physics player"),
            Player,
            RigidBody::Dynamic,
            Collider::capsule(0.3, 1.4),
            Transform::from_translation(position),
            LockedAxes::ROTATION_LOCKED,
            Friction::ZERO,
            Restitution::ZERO,
            LinearVelocity::ZERO,
            GravityScale(1.0),
        ))
        .with_children(|player| {
            // Trigger volumes test the player from the feet up to the top of the
            // capsule, not the camera point alone.
            player.spawn((
                crate::heron_animation::HeronActivator,
                crate::heron_animation::HeronActivatorHeight(2.0),
                Transform::from_xyz(0.0, -1.0, 0.0),
            ));
        });
    for (camera, transform) in &cameras {
        let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        commands.entity(camera).insert(Aim { yaw, pitch });
    }
    spawned.0 = true;
    info!(
        "PHYSICS_READY: {} exported collider parts",
        colliders.iter().count()
    );
}

fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mouse: Res<AccumulatedMouseMotion>,
    args: Res<Args>,
    mut cursors: Query<&mut bevy::window::CursorOptions>,
    mut players: Query<(Entity, &Position, &mut LinearVelocity,&mut GravityScale), With<Player>>,
    waters:Query<(&crate::heron_water::HeronWater,&GlobalTransform)>,
    time:Res<Time>,
    mut cameras: Query<(&mut Transform, &mut Aim), (With<Camera3d>, Without<Player>)>,
    spatial: SpatialQuery,
    sensors: Query<Entity, With<Sensor>>,
) {
    if args.physics_check {
        return;
    }
    let Ok((entity, position, mut velocity,mut gravity)) = players.single_mut() else {
        return;
    };
    let Ok((mut camera, mut aim)) = cameras.single_mut() else {
        return;
    };
    for mut cursor in &mut cursors {
        if buttons.just_pressed(MouseButton::Right) {
            cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
            cursor.visible = false;
        }
        if buttons.just_released(MouseButton::Right) || keys.just_pressed(KeyCode::Escape) {
            cursor.grab_mode = bevy::window::CursorGrabMode::None;
            cursor.visible = true;
        }
    }
    if buttons.pressed(MouseButton::Right) {
        aim.yaw -= mouse.delta.x * 0.003;
        aim.pitch = (aim.pitch - mouse.delta.y * 0.003).clamp(-1.5, 1.5);
    }
    camera.rotation = Quat::from_euler(EulerRot::YXZ, aim.yaw, aim.pitch, 0.0);
    let x = i32::from(keys.pressed(KeyCode::KeyD)) - i32::from(keys.pressed(KeyCode::KeyA));
    let z = i32::from(keys.pressed(KeyCode::KeyS)) - i32::from(keys.pressed(KeyCode::KeyW));
    let direction =
        Quat::from_rotation_y(aim.yaw) * Vec3::new(x as f32, 0.0, z as f32).normalize_or_zero();
    if apply_water(waters.iter(),position.0,&mut velocity.0,&mut gravity.0,direction,keys.pressed(KeyCode::Space),keys.pressed(KeyCode::ControlLeft),time.delta_secs()){return;}
    let speed = if keys.pressed(KeyCode::ShiftLeft) {
        8.0
    } else {
        4.0
    };
    velocity.x = direction.x * speed;
    velocity.z = direction.z * speed;
    let filter = SpatialQueryFilter::from_excluded_entities(sensors.iter().chain([entity]));
    if keys.just_pressed(KeyCode::Space)
        && velocity.y <= 0.1
        && spatial
            .cast_ray(position.0, Dir3::NEG_Y, 1.08, true, &filter)
            .is_some()
    {
        velocity.y = 7.0;
    }
}

fn follow_player(
    players: Query<&Position, With<Player>>,
    mut cameras: Query<&mut Transform, With<Aim>>,
) {
    let Ok(position) = players.single() else {
        return;
    };
    for mut camera in &mut cameras {
        camera.translation = position.0 + Vec3::Y * 0.7;
    }
}

#[derive(Resource, Default)]
struct PlatformPoses(std::collections::HashMap<Entity, Affine3A>);

/// Apply the support's complete pose delta before Avian simulates the player.
/// The ray query still describes the previous physics frame, matching the saved
/// pose. This handles translation and rotation without accumulated angular drift.
fn carry_player(
    platforms: Query<(Entity, &Transform), With<crate::heron_animation::HeronAnimation>>,
    parents: Query<&ChildOf>,
    sensors: Query<Entity, With<Sensor>>,
    mut previous: ResMut<PlatformPoses>,
    mut queries: ParamSet<(
        SpatialQuery,
        Query<
            (Entity, &mut Position, &mut Transform, &LinearVelocity),
            (
                With<Player>,
                Without<crate::heron_animation::HeronAnimation>,
            ),
        >,
    )>,
) {
    let current: std::collections::HashMap<_, _> = platforms
        .iter()
        .map(|(entity, transform)| (entity, transform.compute_affine()))
        .collect();
    let players: Vec<_> = queries
        .p1()
        .iter()
        .map(|(entity, position, _, velocity)| (entity, position.0, velocity.y))
        .collect();
    for (player, position, vertical_velocity) in players {
        // An upward velocity means the player has jumped and must detach.
        if vertical_velocity > 0.1 {
            continue;
        }
        let filter = SpatialQueryFilter::from_excluded_entities(sensors.iter().chain([player]));
        let Some(hit) = queries
            .p0()
            .cast_ray(position, Dir3::NEG_Y, 1.08, true, &filter)
        else {
            continue;
        };
        let mut support = hit.entity;
        while !current.contains_key(&support) {
            let Ok(parent) = parents.get(support) else {
                break;
            };
            support = parent.parent();
        }
        if let (Some(old), Some(new)) = (previous.0.get(&support), current.get(&support)) {
            let carried = new.transform_point3(old.inverse().transform_point3(position));
            let mut players = queries.p1();
            if let Ok((_, mut position, mut transform, _)) = players.get_mut(player) {
                position.0 = carried;
                transform.translation = carried;
            }
        }
    }
    previous.0 = current;
}

#[derive(Resource, Default)]
struct PhysicsCheck {
    time: f32,
    phase: u8,
    probe: Option<Entity>,
    sensor: Option<Entity>,
    entered: bool,
    initial: Vec3,
}
#[derive(Component)]
struct CheckProbe;

fn check_physics(
    mut commands: Commands,
    args: Res<Args>,
    time: Res<Time>,
    spawned: Res<PlayerSpawned>,
    mut check: ResMut<PhysicsCheck>,
    players: Query<(&Position, &LinearVelocity), With<Player>>,
    sensors: Query<(Entity, &GlobalTransform, &CollidingEntities), With<Sensor>>,
    probes: Query<(&Position, &LinearVelocity), With<CheckProbe>>,
    spatial: SpatialQuery,
    roots: Query<(&crate::SourceEntity, &Children)>,
    level: Res<level::Level>,
    mut exit: MessageWriter<AppExit>,
) {
    if !args.physics_check {
        return;
    }
    check.time += time.delta_secs();
    if check.time > 45.0 {
        error!("PHYSICS_CHECK failed: timeout at phase {}", check.phase);
        exit.write(AppExit::error());
        return;
    }
    if !spawned.0 {
        return;
    }
    if check.phase == 0 {
        check.time = 0.0;
        check.phase = 1;
    }
    if check.phase == 1 && check.time > 2.0 {
        let Ok((position, velocity)) = players.single() else {
            return;
        };
        let filter = SpatialQueryFilter::from_excluded_entities(sensors.iter().map(|s| s.0));
        // Start outside the capsule so the ray must hit exported world geometry.
        let floor = spatial.cast_ray(position.0 + Vec3::X * 0.5, Dir3::NEG_Y, 1.2, true, &filter);
        if velocity.y.abs() > 0.1 || floor.is_none() {
            error!(
                "PHYSICS_CHECK failed: player did not settle on exported floor: {:?} {:?}",
                position, velocity
            );
            exit.write(AppExit::error());
            return;
        }
        info!(
            "PHYSICS_CHECK PASS: dynamic capsule rests on exported triangle mesh at {:?}",
            position.0
        );
        let sensor_id = roots
            .iter()
            .find(|(source, _)| level.entities[source.0].classname == "trigger_multiple")
            .and_then(|(_, children)| children.iter().find(|child| sensors.contains(*child)));
        let Some((sensor, transform, _)) = sensor_id.and_then(|id| sensors.get(id).ok()) else {
            error!("PHYSICS_CHECK failed: no exported sensor");
            exit.write(AppExit::error());
            return;
        };
        let initial = transform.translation();
        let probe = commands
            .spawn((
                CheckProbe,
                RigidBody::Dynamic,
                Collider::sphere(0.1),
                GravityScale(0.0),
                LockedAxes::ROTATION_LOCKED,
                Transform::from_translation(initial),
                LinearVelocity(Vec3::Y * 2.0),
            ))
            .id();
        check.probe = Some(probe);
        check.sensor = Some(sensor);
        check.initial = initial;
        check.phase = 2;
        check.time = 0.0;
    }
    if check.phase == 2 {
        if let (Some(probe), Some(sensor)) = (check.probe, check.sensor) {
            if let Ok((_, _, overlaps)) = sensors.get(sensor) {
                check.entered |= overlaps.contains(&probe);
            }
            if check.time > 2.0 {
                let Ok((position, velocity)) = probes.get(probe) else {
                    return;
                };
                let exited = sensors.get(sensor).is_ok_and(|s| !s.2.contains(&probe));
                if !check.entered
                    || !exited
                    || position.y < check.initial.y + 3.0
                    || velocity.y < 1.9
                {
                    error!(
                        "PHYSICS_CHECK failed: sensor enter={} exit={} position={:?} velocity={:?}",
                        check.entered, exited, position, velocity
                    );
                    exit.write(AppExit::error());
                    return;
                }
                info!("PHYSICS_CHECK PASS: exported sensor reports enter and exit without blocking a dynamic body");
                info!("PHYSICS_CHECK PASS");
                check.phase = 3;
                exit.write(AppExit::Success);
            }
        }
    }
}

#[cfg(test)]
#[path = "motion_check.rs"]
mod motion_check;

#[cfg(test)]
#[path = "feature_check.rs"]
mod feature_check;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{ecs::system::RunSystemOnce, time::TimeUpdateStrategy};
    pub(super) fn app() -> App {
        let mut app = app_before_finish();
        app.finish();
        app.cleanup();
        app
    }
    pub(super) fn app_before_finish() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ));
        app.insert_resource(Time::<Fixed>::from_hz(60.0));
        app
    }
    pub(super) fn tick(app: &mut App, frames: usize) {
        for _ in 0..frames {
            app.update();
        }
    }

    fn stationary_animation() -> crate::heron_animation::HeronAnimation {
        crate::heron_animation::HeronAnimation::from_json(r#"{"version":1,"scale":32,"pivot":[0,0,0],"selected":"test","autoplay":false,"clips":[{"name":"test","mode":"loop","duration":2,"channels":[{"keys":[]},{"keys":[]},{"keys":[]},{"keys":[]},{"keys":[]},{"keys":[]}]}]}"#).unwrap()
    }

    #[test]
    fn player_rides_translating_and_rotating_support_without_drift() {
        let mut app = app();
        app.init_resource::<PlatformPoses>()
            .add_systems(FixedPreUpdate, carry_player);
        let platform = app
            .world_mut()
            .spawn((
                RigidBody::Kinematic,
                Transform::from_xyz(0.0, -0.25, 0.0),
                stationary_animation(),
            ))
            .id();
        app.world_mut().spawn((
            Collider::cuboid(8.0, 0.5, 8.0),
            Transform::default(),
            ChildOf(platform),
        ));
        let player = app
            .world_mut()
            .spawn((
                Player,
                RigidBody::Dynamic,
                Collider::capsule(0.3, 1.4),
                Transform::from_xyz(2.0, 1.05, 0.0),
                LockedAxes::ROTATION_LOCKED,
                Friction::ZERO,
                Restitution::ZERO,
                LinearVelocity::ZERO,
            ))
            .id();
        tick(&mut app, 60);
        let anchor = app
            .world()
            .get::<Transform>(platform)
            .unwrap()
            .compute_affine()
            .inverse()
            .transform_point3(app.world().get::<Position>(player).unwrap().0);
        for frame in 1..=720 {
            let mut pose = app.world_mut().get_mut::<Transform>(platform).unwrap();
            pose.translation.x = frame as f32 / 120.0;
            pose.rotation = Quat::from_rotation_y(frame as f32 / 60.0);
            drop(pose);
            tick(&mut app, 1);
            let local = app
                .world()
                .get::<Transform>(platform)
                .unwrap()
                .compute_affine()
                .inverse()
                .transform_point3(app.world().get::<Position>(player).unwrap().0);
            assert!(
                (local - anchor).length() < 0.04,
                "frame {frame}: {local:?}, anchor {anchor:?}"
            );
        }
        app.world_mut().get_mut::<LinearVelocity>(player).unwrap().y = 7.0;
        let before = app.world().get::<Position>(player).unwrap().0;
        app.world_mut()
            .get_mut::<Transform>(platform)
            .unwrap()
            .translation
            .x += 1.0;
        tick(&mut app, 1);
        let after = app.world().get::<Position>(player).unwrap().0;
        assert!(
            (after.x - before.x).abs() < 0.05,
            "jump must detach from support"
        );
        assert!(after.y > before.y, "jump must rise");
    }

    #[test]
    fn exported_point_cloud_builds_a_hull_without_triangle_indices() {
        let mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::PointList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-1.0f32, -1.0, -1.0],
                [1.0, -1.0, -1.0],
                [-1.0, 1.0, -1.0],
                [1.0, 1.0, -1.0],
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [-1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
            ],
        );
        assert!(mesh.indices().is_none());
        assert!(convex_from_points(&mesh).is_some());
    }

    #[test]
    fn expanded_triangle_mesh_needs_no_index_buffer() {
        let mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[-2.0f32, 0.0, -2.0], [2.0, 0.0, 2.0], [2.0, 0.0, -2.0]],
        );
        assert!(trimesh_from_triangles(&mesh).is_some());
    }

    #[test]
    fn dynamic_import_waits_for_all_async_collision_parts() {
        let mut app = app();
        app.add_systems(Update, activate_bodies);
        let body = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                RigidBodyDisabled,
                AwaitingWorld,
                Transform::from_xyz(0.0, 5.0, 0.0),
                Collider::sphere(0.5),
            ))
            .id();
        let pending = app
            .world_mut()
            .spawn(PendingCollider {
                kind: CollisionShape::Trimesh,
                sensor: false,
            })
            .id();
        tick(&mut app, 60);
        assert!((app.world().get::<Position>(body).unwrap().y - 5.0).abs() < 0.001);
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(20.0, 1.0, 20.0),
            Transform::from_xyz(0.0, -0.5, 0.0),
        ));
        app.world_mut().despawn(pending);
        tick(&mut app, 180);
        let y = app.world().get::<Position>(body).unwrap().y;
        assert!(
            (y - 0.5).abs() < 0.03,
            "body should rest on loaded floor, y={y}"
        );
    }

    #[test]
    fn full_transform_and_moving_parent_update_convex_collision() {
        let mut app = app();
        app.insert_resource(Gravity(Vec3::ZERO));
        let transform = Transform::from_xyz(10.0, 20.0, 30.0)
            .with_rotation(Quat::from_euler(EulerRot::YXZ, 0.7, 0.3, -0.2))
            .with_scale(Vec3::new(2.0, 3.0, 4.0));
        let parent = app
            .world_mut()
            .spawn((RigidBody::Kinematic, transform))
            .id();
        let child = app
            .world_mut()
            .spawn((
                Collider::convex_hull(vec![
                    Vec3::new(-1.0, -1.0, -1.0),
                    Vec3::new(1.0, -1.0, -1.0),
                    Vec3::new(-1.0, 1.0, -1.0),
                    Vec3::new(1.0, 1.0, -1.0),
                    Vec3::new(-1.0, -1.0, 1.0),
                    Vec3::new(1.0, -1.0, 1.0),
                    Vec3::new(-1.0, 1.0, 1.0),
                    Vec3::new(1.0, 1.0, 1.0),
                ])
                .unwrap(),
                Transform::default(),
                ChildOf(parent),
            ))
            .id();
        tick(&mut app, 5);
        app.world_mut()
            .run_system_once(move |query: SpatialQuery| {
                let direction = transform.rotation * Vec3::X;
                let hit = query
                    .cast_ray(
                        transform.translation + direction * 10.0,
                        Dir3::new(-direction).unwrap(),
                        20.0,
                        true,
                        &SpatialQueryFilter::default(),
                    )
                    .unwrap();
                assert_eq!(hit.entity, child);
                assert!(
                    (hit.distance - 8.0).abs() < 0.02,
                    "scaled hull distance {}",
                    hit.distance
                );
            })
            .unwrap();
        app.world_mut()
            .entity_mut(parent)
            .insert(LinearVelocity(Vec3::X * 6.0));
        tick(&mut app, 60);
        let moved = app.world().get::<Position>(parent).unwrap().0;
        assert!((moved.x - 16.0).abs() < 0.15, "{moved:?}");
        app.world_mut()
            .run_system_once(move |query: SpatialQuery| {
                let hit = query
                    .cast_ray(
                        moved + Vec3::Y * 10.0,
                        Dir3::NEG_Y,
                        20.0,
                        true,
                        &SpatialQueryFilter::default(),
                    )
                    .unwrap();
                assert_eq!(hit.entity, child);
            })
            .unwrap();
    }
}

fn apply_water<'a>(waters:impl Iterator<Item=(&'a crate::heron_water::HeronWater,&'a GlobalTransform)>,point:Vec3,velocity:&mut Vec3,gravity:&mut f32,walk:Vec3,rise:bool,dive:bool,dt:f32)->bool{
    let mut wet:Option<(&crate::heron_water::HeronWater,f32)>=None;
    for (water,pose) in waters {if let Some(depth)=water.sample(pose,point){if wet.is_none_or(|(old,d)|water.0.priority>old.0.priority||(water.0.priority==old.0.priority&&depth>d)){wet=Some((water,depth));}}}
    *gravity=if wet.is_some(){0.}else{1.};
    if let Some((water,_))=wet{*velocity=water.velocity(*velocity,walk,rise,dive,dt);true}else{false}
}
#[cfg(test)]
mod water_tests {
 use super::*;
 #[derive(Resource,Default)]struct Input{rise:bool,dive:bool,walk:Vec3}
 fn swim(waters:Query<(&crate::heron_water::HeronWater,&GlobalTransform)>,mut players:Query<(&Position,&mut LinearVelocity,&mut GravityScale),With<Player>>,input:Res<Input>){for (p,mut v,mut gravity) in &mut players{apply_water(waters.iter(),p.0,&mut v.0,&mut gravity.0,input.walk,input.rise,input.dive,1./60.);}}
 #[test]
 #[ignore="requires the fresh core water export at /tmp/heron-water-check"]
 fn exported_water_native_avian_swimming_and_parent_transform(){
  let json=std::fs::read_to_string("/tmp/heron-water-check/water-data.json").unwrap();let water=crate::heron_water::HeronWater::from_json(&json).unwrap();
  let mut app=super::tests::app();app.init_resource::<Input>().insert_resource(Gravity(Vec3::NEG_Y*22.)).add_systems(FixedPreUpdate,swim);
  let data:serde_json::Value=serde_json::from_str(&std::fs::read_to_string("/tmp/heron-water-check/bevy/water.json").unwrap()).unwrap();let source=data["entities"].as_array().unwrap().iter().find(|e|e["name"]=="water_volume").unwrap();let t=&source["transform"];let position=Vec3::new(t["translation"][0].as_f64().unwrap()as f32,t["translation"][1].as_f64().unwrap()as f32,t["translation"][2].as_f64().unwrap()as f32);
  let root=app.world_mut().spawn((water.clone(),RigidBody::Static,Transform::from_translation(position))).id();
  app.world_mut().spawn((Collider::cuboid(8.,4.,8.),Sensor,Transform::from_xyz(0.,-2.,0.),ChildOf(root)));
  app.world_mut().spawn((RigidBody::Static,Collider::cuboid(20.,1.,20.),Transform::from_xyz(0.,-4.5,0.)));
  let player=app.world_mut().spawn((Player,RigidBody::Dynamic,Collider::capsule(0.3,1.4),LockedAxes::ROTATION_LOCKED,GravityScale(1.),LinearVelocity::ZERO,Transform::from_xyz(0.,-2.,0.))).id();super::tests::tick(&mut app,2);
  assert_eq!(app.world().get::<GravityScale>(player).unwrap().0,0.);app.world_mut().resource_mut::<Input>().rise=true;super::tests::tick(&mut app,20);let up=app.world().get::<Position>(player).unwrap().y;assert!(up> -1.75,"rise {up}");
  app.world_mut().resource_mut::<Input>().rise=false;app.world_mut().resource_mut::<Input>().dive=true;super::tests::tick(&mut app,35);let down=app.world().get::<Position>(player).unwrap().y;assert!(down<up-0.3,"dive {up} -> {down}");
  app.world_mut().resource_mut::<Input>().dive=false;app.world_mut().resource_mut::<Input>().walk=Vec3::X;super::tests::tick(&mut app,20);assert!(app.world().get::<Position>(player).unwrap().x>0.3);
  app.world_mut().get_mut::<Position>(player).unwrap().0=Vec3::new(8.,1.,0.);super::tests::tick(&mut app,2);assert_eq!(app.world().get::<GravityScale>(player).unwrap().0,1.);
  let pose=GlobalTransform::from(Transform::from_xyz(7.,2.,3.).with_rotation(Quat::from_rotation_y(0.7)).with_scale(Vec3::splat(1.5)));assert!(water.sample(&pose,pose.transform_point(Vec3::ZERO)).is_some());
  println!("WATER_CHECK Bevy PASS native Avian rise dive swim sensor exit gravity parent");
 }
}
