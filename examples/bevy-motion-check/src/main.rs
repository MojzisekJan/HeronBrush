//! Headless checks of the real Bevy sample adapter, without renderer/UI/winit.
#![allow(dead_code)]
#[path = "../../../demos/bevy_fps/src/level.rs"]
mod level;
#[path = "../../../demos/bevy_fps/src/physics.rs"]
mod physics;
#[path = "../../../crates/scene_export/src/writer/bevy_water.rs.txt"]
mod heron_water;
#[path = "../../../crates/scene_export/src/writer/bevy_animation.rs.txt"]
mod heron_animation;
#[path = "../../../crates/scene_export/src/writer/bevy_entities.rs.txt"]
mod heron_entities;

use bevy::prelude::*;

// The adapter only reads these two switches from the viewer's arguments.
#[derive(Resource)]
struct Args {fly: bool, physics_check: bool}
#[derive(Component)]
struct SourceEntity(usize);

fn main() {
    println!("Run cargo test --manifest-path examples/bevy-motion-check/Cargo.toml. Export motion_lab first for the ignored runtime test.");
}
