//! Feature lab regression: each exported GLB's collision mesh through Avian, the
//! trigger sensor opening the door, heron_animation.rs against the editor's poses
//! and the water adapter. Export with examples/engine-checks/export_feature_lab.py
//! and write the poses with feature_lab_poses.py first.
use super::motion_check::exported_colliders_for;
use super::tests::{app_before_finish, tick};
use super::*;
use crate::heron_animation::{
    HeronActivator, HeronActivatorHeight, HeronAnimation, HeronAnimationFixedPlugin,
};
use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::ecs::system::RunSystemOnce;
use std::collections::HashMap;
use std::path::PathBuf;

fn setting(name: &str, fallback: &str) -> PathBuf {
    std::env::var_os(name).map_or_else(|| fallback.into(), PathBuf::from)
}

/// How far down from y = 10 a ray at the entity's position meets something solid.
fn floor_under(app: &mut App, at: Vec3) -> f32 {
    app.world_mut()
        .run_system_once(
            move |query: SpatialQuery, sensors: Query<Entity, With<Sensor>>| {
                let filter = SpatialQueryFilter::default().with_excluded_entities(&sensors);
                let origin = Vec3::new(at.x, 10.0, at.z);
                query
                    .cast_ray(origin, Dir3::NEG_Y, 20.0, true, &filter)
                    .map_or(f32::NAN, |hit| 10.0 - hit.distance)
            },
        )
        .expect("the ray query runs")
}

#[test]
#[ignore = "export the feature lab with examples/engine-checks/export_feature_lab.py bevy first"]
fn exported_feature_lab_collision_triggers_animation_and_water() {
    let directory = setting("HERON_BEVY_FEATURE", "/tmp/heron-feature-bevy");
    let poses: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(setting(
            "HERON_FEATURE_POSES",
            "/tmp/feature-lab-poses.json",
        ))
        .expect("feature_lab_poses.py has written the poses"),
    )
    .expect("the poses are JSON");
    let level = level::load(&directory.join("feature_lab.json")).expect("the level loads");
    let named: HashMap<String, &level::Entity> = level
        .entities
        .iter()
        .filter_map(|e| Some((e.properties.get("targetname")?.as_str()?.to_string(), e)))
        .collect();
    let mut app = app_before_finish();
    app.insert_resource(Gravity(Vec3::NEG_Y * 22.0))
        .add_plugins(HeronAnimationFixedPlugin);
    app.finish();
    app.cleanup();
    let roots = exported_colliders_for(&mut app, &directory, "feature_lab.json");
    let root = |name: &str| roots[&named[name].name];
    let at = |name: &str| Vec3::from(named[name].transform.translation);
    tick(&mut app, 2);

    // Collision from each GLB's collision mesh: hulls and triangles, sensors apart.
    let solid = floor_under(&mut app, at("fl_solid"));
    let clip = floor_under(&mut app, at("fl_clip"));
    let skip = floor_under(&mut app, at("fl_skip"));
    let hidden = floor_under(&mut app, at("fl_hidden"));
    let trigger = floor_under(&mut app, at("fl_trigger"));
    println!(
        "collision: rays stop at fl_solid {solid:.3}, fl_clip {clip:.3}, fl_hidden {hidden:.3}; \
         through fl_skip's top to {skip:.3}; through the trigger sensor to {trigger:.3}"
    );
    assert!((solid - 2.25).abs() < 1e-3 && (clip - 3.25).abs() < 1e-3);
    assert!((hidden - 2.25).abs() < 1e-3 && skip < 2.2 && trigger.abs() < 1e-3);
    let sensors = app
        .world_mut()
        .query_filtered::<&ChildOf, With<Sensor>>()
        .iter(app.world())
        .map(|parent| parent.parent())
        .collect::<Vec<_>>();
    assert!(sensors.contains(&root("fl_trigger")) && sensors.contains(&root("fl_water")));

    // The editor's poses: each probe point, moved by the root's pose against its rest.
    let point = |v: &serde_json::Value| {
        Vec3::new(
            v[0].as_f64().expect("x") as f32,
            v[1].as_f64().expect("y") as f32,
            v[2].as_f64().expect("z") as f32,
        )
    };
    // The largest distance between the editor's posed probes of one sample and the
    // same rest probes moved by the entity's pose now.
    let error = |app: &App, name: &str, sample: &serde_json::Value| {
        let rest = named[name].transform.to_transform().compute_affine();
        let now = app
            .world()
            .get::<Transform>(root(name))
            .expect("a transform")
            .compute_affine();
        sample["rest"]
            .as_array()
            .expect("rest points")
            .iter()
            .zip(sample["posed"].as_array().expect("posed points"))
            .map(|(r, p)| {
                now.transform_point3(rest.inverse().transform_point3(point(r)))
                    .distance(point(p))
            })
            .fold(0.0_f32, f32::max)
    };
    let mut worst = 0.0_f32;
    for (name, clip) in [("fl_mover", None), ("fl_door", Some("open"))] {
        let entity = root(name);
        for sample in poses[name].as_array().expect("samples") {
            let time = sample["time"].as_f64().expect("a time");
            {
                let mut animation = app
                    .world_mut()
                    .get_mut::<HeronAnimation>(entity)
                    .expect("an animation");
                if let Some(clip) = clip {
                    animation.play(clip);
                }
                animation.seek(time);
                animation.pause();
            }
            tick(&mut app, 1);
            worst = worst.max(error(&app, name, sample));
        }
        app.world_mut()
            .get_mut::<HeronAnimation>(entity)
            .expect("an animation")
            .stop();
    }
    println!("animation: heron_animation.rs matches the editor's poses within {worst:.1e} m");
    assert!(worst < 1e-4, "pose error {worst}");
    tick(&mut app, 2);
    let open = &poses["fl_door"].as_array().expect("door samples")[6];
    let closed = &poses["fl_door"].as_array().expect("door samples")[0];
    assert!(
        error(&app, "fl_door", closed) < 1e-4,
        "the door starts closed"
    );

    // A player standing in the raised trigger: the sensor sees it, and the activator
    // from its feet up opens the door.
    let feet = Vec3::new(at("fl_trigger").x, 0.02, at("fl_trigger").z);
    let player = app
        .world_mut()
        .spawn((
            Player,
            RigidBody::Dynamic,
            Collider::capsule(0.3, 1.4),
            Transform::from_translation(feet + Vec3::Y),
            LockedAxes::ROTATION_LOCKED,
            LinearVelocity::ZERO,
        ))
        .id();
    app.world_mut().spawn((
        HeronActivator,
        HeronActivatorHeight(2.0),
        Transform::from_xyz(0.0, -1.0, 0.0),
        ChildOf(player),
    ));
    tick(&mut app, 90);
    let touching = app
        .world_mut()
        .query::<(&ChildOf, &CollidingEntities)>()
        .iter(app.world())
        .any(|(parent, colliding)| {
            parent.parent() == root("fl_trigger") && colliding.contains(&player)
        });
    let opened = error(&app, "fl_door", open);
    let door = app
        .world()
        .get::<HeronAnimation>(root("fl_door"))
        .expect("the door animates");
    println!(
        "trigger: sensor reports the player {touching}; after 1.5 s the door's clip is {} \
         and it is {opened:.1e} m from the editor's open pose",
        door.current_clip()
    );
    assert!(touching && door.current_clip() == "open" && !door.is_playing() && opened < 1e-4);

    // Water: the adapter's depth at the volume's centre.
    let water = crate::heron_water::HeronWater::from_json(
        named["fl_water"].properties["_heron_water"]
            .as_str()
            .expect("water data"),
    )
    .expect("the water payload reads");
    let pose = *app
        .world()
        .get::<GlobalTransform>(root("fl_water"))
        .expect("a pose");
    let depth = water.sample(&pose, at("fl_water"));
    let above = water.sample(&pose, at("fl_water") + Vec3::Y);
    println!("water: {depth:?} m deep at the centre, {above:?} above the surface");
    assert!(depth.is_some_and(|d| (d - 0.75).abs() < 1e-4) && above.is_none());
    println!("FEATURE_LAB Bevy PASS");
}
