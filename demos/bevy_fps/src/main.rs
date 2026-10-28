//! A Heron Brush export in Bevy with Avian collision and a walking camera.
//!
//! ```sh
//! cargo run --release                      # assets/maps/demo.json
//! cargo run --release -- maps/other.heron_brush.json
//! ```
//!
//! There is no heron_brush plugin and no import step. The level file says which `.glb`
//! primitive wears which texture, and this reads it: one entity per solid, one
//! child per surface, a material per texture. What a `func_door` or an
//! `fps_pickup` turns into is the game's business, and this demo, being a
//! sample, imports their geometry and collision without prescribing their gameplay.

mod level;
mod physics;
#[path="../../../crates/scene_export/src/writer/bevy_water.rs.txt"]
mod heron_water;
#[path="../../../crates/scene_export/src/writer/bevy_water_render.rs.txt"]
mod heron_water_render;
#[path = "../../../crates/scene_export/src/writer/bevy_animation.rs.txt"]
mod heron_animation;
#[path = "../../../crates/scene_export/src/writer/bevy_entities.rs.txt"]
mod heron_entities;
#[path = "../../../crates/scene_export/src/writer/bevy_lights.rs.txt"]
mod heron_lights;
#[path = "../../../crates/scene_export/src/writer/bevy_sky.rs.txt"]
mod heron_sky;

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    light::CascadeShadowConfigBuilder,
    prelude::*,
    render::view::screenshot::{save_to_disk, Capturing, Screenshot},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// What the asset server reads from, and what the level file's texture paths
/// are relative to.
const ASSETS: &str = "assets";

const DEFAULT_LEVEL: &str = "maps/demo.json";

fn main() -> AppExit {
    let args = match Args::parse(std::env::args().skip(1)) {
        Ok(Some(args)) => args,
        Ok(None) => return AppExit::Success,
        Err(message) => {
            eprintln!("{message}");
            return AppExit::error();
        }
    };

    let level = match level::load(&asset_root().join(&args.level)) {
        Ok(level) => level,
        Err(message) => {
            eprintln!("{message}");
            if std::env::var_os("CARGO_MANIFEST_DIR").is_none()
                && std::env::var_os("BEVY_ASSET_ROOT").is_none()
            {
                eprintln!(
                    "assets are looked for next to the binary when cargo is not the one \
                     starting it. Set BEVY_ASSET_ROOT to the folder that holds `assets/`."
                );
            }
            return AppExit::error();
        }
    };

    // Meshes are named relative to the level file, textures relative to the
    // project root, so the two are resolved against different places.
    let mesh_dir = args.level.parent().unwrap_or(Path::new("")).to_path_buf();

    println!(
        "{} by {}: {} entities, {} textures, {} map units to the metre",
        level.name,
        level.generator,
        level.entities.len(),
        level.textures.len(),
        level.scale
    );

    // An exported sky lights the level in the editor's units, under its own exposure;
    // the demo's daylight fill would wash that out.
    let sky = match level.environment.as_ref().map(heron_sky::HeronSky::from_json).transpose() {
        Ok(sky) => sky,
        Err(message) => {
            eprintln!("the level's environment: {message}");
            return AppExit::error();
        }
    };
    let ambient = if sky.is_some() { 0.0 } else { 500.0 };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("{} (Heron Brush)", level.name),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FreeCameraPlugin)
        .add_plugins(physics::LevelPhysicsPlugin)
        .insert_resource(ClearColor(Color::srgb(0.45, 0.62, 0.85)))
        // The level file says nothing about ambient light. This is a sky's
        // worth of fill: enough to see into a shadow, not enough to flatten
        // the sun out.
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.75, 0.85, 1.0),
            brightness: ambient,
            ..default()
        })
        .insert_resource(MeshDir(mesh_dir))
        .insert_resource(Sky(sky))
        .insert_resource(level)
        .insert_resource(args)
        .add_plugins((heron_entities::HeronEntityPlugin, heron_lights::HeronLightPlugin))
        .add_plugins(heron_water_render::HeronWaterPlugin)
        .add_systems(Update, prepare_water_surfaces)
        .add_plugins(heron_animation::HeronAnimationFixedPlugin)
        .add_systems(FixedPreUpdate, attach_animations.before(heron_animation::HeronAnimationSet))
        .add_systems(Startup, (spawn_level, heron_entities::configure_entities).chain())
        .add_systems(Startup, (spawn_camera, spawn_hud))
        .add_systems(
            Update,
            (
                toggle_markers,
                toggle_collision,
                update_readout,
                screenshot_on_key,
                auto_screenshot,
            ),
        )
        .run()
}

/// Where `assets/` is, worked out the way Bevy's own file reader works it out,
/// so the level file and the meshes it names are read from the same place
/// whether this was started by cargo or run as a binary.
fn asset_root() -> PathBuf {
    let base = std::env::var_os("BEVY_ASSET_ROOT")
        .or_else(|| std::env::var_os("CARGO_MANIFEST_DIR"))
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf))
        })
        .unwrap_or_default();
    base.join(ASSETS)
}

/// Where the level file sits under `assets/`, which is what its `mesh` paths
/// are relative to.
#[derive(Resource)]
struct MeshDir(PathBuf);

/// The exported sky, if the level has one.
#[derive(Resource)]
struct Sky(Option<heron_sky::HeronSky>);

#[derive(Resource)]
struct Args {
    /// The level file, under `assets/`.
    level: PathBuf,
    /// Write one screenshot and quit. For checking the thing renders with
    /// nobody at the keyboard.
    shot: Option<PathBuf>,
    /// Frames to draw first. Meshes and textures load while the window is
    /// already up, so a shot on frame one is of an empty blue room.
    frames: u32,
    /// Where to put the camera, in metres, instead of at the player start. It
    /// looks at the middle of the map from there.
    at: Option<Vec3>,
    /// Which way the camera at `at` looks, instead of at the middle of the map.
    look: Option<Vec3>,
    /// Show the collision overlay from the first frame, so `--shot` can catch
    /// it. C toggles it either way.
    collision: bool,
    fly: bool,
    physics_check: bool,
}

impl Args {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Option<Self>, String> {
        let mut parsed = Args {
            level: PathBuf::from(DEFAULT_LEVEL),
            shot: None,
            frames: 240,
            at: None,
            look: None,
            collision: false,
            fly: false,
            physics_check: false,
        };
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    println!(
                        "usage: heron_brush-bevy-demo [<level.heron_brush.json>]\n\
                         \x20      [--at <x,y,z> [--look <x,y,z>]] [--fly] [--physics-check] [--collision] [--shot <png> [--frames <n>]]\n\
                         \n\
                         The level file is read from under `assets/`, and defaults to\n\
                         {DEFAULT_LEVEL}.\n\
                         \n\
                         --at puts the camera there, in metres, looking at the middle of the\n\
                         map, which is how the pictures in the README were taken. --look\n\
                         turns it along a direction instead, such as the sun's.\n\
                         \n\
                         Hold right mouse to look, WASD walks, Space jumps. With --fly, Q/E move vertically,\n\
                         shift runs, the wheel changes speed. P hides the markers on point\n\
                         entities, C shows the collision geometry, F12 writes a screenshot."
                    );
                    return Ok(None);
                }
                "--shot" => {
                    parsed.shot = Some(PathBuf::from(
                        args.next().ok_or("--shot wants a file to write")?,
                    ));
                }
                "--at" => {
                    let text = args.next().ok_or("--at wants x,y,z in metres")?;
                    let numbers: Result<Vec<f32>, _> =
                        text.split(',').map(str::trim).map(str::parse).collect();
                    match numbers.as_deref() {
                        Ok([x, y, z]) => parsed.at = Some(Vec3::new(*x, *y, *z)),
                        _ => return Err(format!("--at {text}: wanted x,y,z in metres")),
                    }
                }
                "--look" => {
                    let text = args.next().ok_or("--look wants x,y,z")?;
                    let numbers: Result<Vec<f32>, _> =
                        text.split(',').map(str::trim).map(str::parse).collect();
                    match numbers.as_deref() {
                        Ok([x, y, z]) => parsed.look = Some(Vec3::new(*x, *y, *z)),
                        _ => return Err(format!("--look {text}: wanted a direction x,y,z")),
                    }
                }
                "--frames" => {
                    parsed.frames = args
                        .next()
                        .ok_or("--frames wants a number")?
                        .parse()
                        .map_err(|e| format!("--frames: {e}"))?;
                }
                "--collision" => parsed.collision = true,
                "--fly" => parsed.fly = true,
                "--physics-check" => parsed.physics_check = true,
                other if other.starts_with('-') => return Err(format!("unknown option {other}")),
                other => parsed.level = PathBuf::from(other),
            }
        }
        parsed.fly |= parsed.at.is_some();
        if parsed.physics_check && (parsed.fly || parsed.shot.is_some()) {
            return Err("--physics-check runs independently of --fly, --at and --shot".into());
        }
        Ok(Some(parsed))
    }
}

/// Links a spawned root to its complete exported data in the Level resource.
/// Game systems can read custom properties and targets without classname hardcoding
/// in the importer. Index is local to this loaded level, not a persistent save-game ID.
#[derive(Component, Clone, Copy, Debug)]
pub struct SourceEntity(pub usize);

/// A box standing in for a point entity, which a real game would replace with
/// whatever the classname means to it.
#[derive(Component)]
struct Marker;

/// A collision hull or trimesh, drawn as a translucent overlay, hidden until C.
#[derive(Component)]
struct CollisionView;

#[derive(Component)]
struct Readout;

fn spawn_level(
    mut commands: Commands,
    level: Res<level::Level>,
    sky: Res<Sky>,
    mesh_dir: Res<MeshDir>,
    args: Res<Args>,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let collision_visibility = if args.collision {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let mut surfaces: HashMap<&str, Handle<StandardMaterial>> = HashMap::new();
    let mut images: HashMap<&str, Option<Handle<Image>>> = HashMap::new();
    for texture in &level.textures {
        let image = texture.image.as_ref().map(|path| load_image(&assets, path, texture.clamp, true));
        let mut painted = material(image.clone());
        // A brush texture's normal map; the GLB primitives carry the tangents.
        if let Some((_, path)) = texture.maps.iter().find(|(slot, _)| slot == "normal_texture") {
            painted.normal_map_texture = Some(load_image(&assets, path, false, false));
        }
        if let Some([roughness, metallic, emission]) = texture.surface {
            painted.perceptual_roughness = roughness;
            painted.metallic = metallic;
            painted.emissive = LinearRgba::rgb(emission, emission, emission);
            painted.emissive_texture = image.clone();
        }
        surfaces.insert(&texture.name, materials.add(painted));
        images.insert(&texture.name, image);
    }
    let mut underwater_surfaces: HashMap<(usize,&str), Handle<StandardMaterial>> = HashMap::new();

    let marker_mesh = meshes.add(Cuboid::from_length(0.3));
    let mut marker_materials: HashMap<&str, Handle<StandardMaterial>> = HashMap::new();

    // One flat green wash for every collision hull and trimesh, unlit so it
    // reads the same from any angle. Hidden until C.
    let collision_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 1.0, 0.4, 0.3),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        double_sided: true,
        ..default()
    });

    let (mut solids, mut points) = (0, 0);

    for (source_index, entity) in level.entities.iter().enumerate() {
        let placement = entity.transform.to_transform();
        let data=heron_entities::HeronEntityData::from_json(&serde_json::to_value(entity).expect("level entity serializes")).expect("validated level entity");

        // The exported sky brings the editor's own sun.
        if entity.classname == "light_sun" {
            commands.spawn((
                Name::new(entity.name.clone()),
                SourceEntity(source_index),
                data,
                sun(entity),
                placement,
                CascadeShadowConfigBuilder {
                    num_cascades: 4,
                    maximum_distance: entity.float("directional_shadow_max_distance", 100.0),
                    first_cascade_far_bound: 8.0,
                    ..default()
                }
                .build(),
            ));
            continue;
        }

        if entity.classname == "light_spot" {
            commands.spawn((Name::new(entity.name.clone()),SourceEntity(source_index),data,placement));
            continue;
        }
        if entity.classname == "light" {
            let c = entity.colour("light_color").unwrap_or([1.0; 4]);
            commands.spawn((
                Name::new(entity.name.clone()),
                SourceEntity(source_index),
                data,
                placement,
                PointLight {
                    color: Color::srgba(c[0], c[1], c[2], c[3]),
                    intensity: entity.float("light_energy", 1.0) * 1000.0,
                    range: entity.float("omni_range", 12.0),
                    shadow_maps_enabled: entity.float("shadow_enabled", 1.0) != 0.0,
                    ..default()
                },
            ));
            continue;
        }

        // Solids: some have a visual mesh, some only collision (`clip` brushes,
        // trigger volumes), some both. Anything with a `.glb` comes through here.
        if let Some(mesh) = entity.mesh.as_ref().filter(|_| entity.solid) {
            let path = mesh_dir.0.join(mesh).to_string_lossy().into_owned();
            // Water is a volume the player swims in, and a viewer should be
            // able to see the bottom of it. This is the one place the demo
            // reads a classname and decides something.
            let water=entity.properties.get("_heron_water").and_then(level::Value::as_str).and_then(|json|heron_water::HeronWater::from_json(json).ok());
            let underwater=water.as_ref().is_some_and(|w|w.0.surface_material);
            let mut root=commands
                .spawn((
                    Name::new(entity.name.clone()),
                    SourceEntity(source_index),
                data,
                    physics::body_for(entity),
                    avian3d::prelude::RigidBodyDisabled,
                    physics::AwaitingWorld,
                    placement,
                    Visibility::default(),
                ));
            if let Some(water)=water.clone(){root.insert(water);}
            root.with_children(|solid| {
                    for surface in &entity.surfaces {
                        let Some(material) = surfaces.get(surface.texture.as_str()) else {
                            warn!("{}: no texture called {}", entity.name, surface.texture);
                            continue;
                        };
                        let material = if underwater {
                            underwater_surfaces
                                .entry((source_index,surface.texture.as_str()))
                                .or_insert_with(|| {
                                    let image =
                                        images.get(surface.texture.as_str()).cloned().flatten();
                                    materials.add(water.as_ref().unwrap().material(image))
                                })
                                .clone()
                        } else {
                            material.clone()
                        };
                        solid.spawn((
                            Mesh3d(
                                assets.load(
                                    GltfAssetLabel::Primitive {
                                        mesh: 0,
                                        primitive: surface.primitive,
                                    }
                                    .from_asset(path.clone()),
                                ),
                            ),
                            MeshMaterial3d(material),
                        ));
                    }

                    // Collision geometry drives Avian and can be inspected with C.
                    if let Some(collision) = &entity.collision {
                        for primitive in 0..collision.parts {
                            let part = GltfAssetLabel::Primitive {
                                mesh: collision.mesh,
                                primitive,
                            }
                            .from_asset(path.clone());
                            solid.spawn((
                                CollisionView,
                                physics::PendingCollider {
                                    kind: collision.kind,
                                    sensor: collision.sensor,
                                },
                                collision_visibility,
                                Mesh3d(assets.load(part)),
                                MeshMaterial3d(collision_material.clone()),
                            ));
                        }
                    }
                });
            if !entity.surfaces.is_empty() {
                solids += 1;
            }
        } else if !entity.solid {
            let material = marker_materials
                .entry(&entity.classname)
                .or_insert_with(|| {
                    materials.add(StandardMaterial {
                        base_color: classname_colour(&entity.classname),
                        emissive: classname_colour(&entity.classname).to_linear() * 0.4,
                        ..default()
                    })
                })
                .clone();
            commands.spawn((
                Name::new(entity.name.clone()),
                SourceEntity(source_index),
                data,
                Marker,
                Mesh3d(marker_mesh.clone()),
                MeshMaterial3d(material),
                placement,
            ));
            points += 1;
        } else {
            // A brush used solely as project metadata still needs a native root.
            commands.spawn((Name::new(entity.name.clone()),SourceEntity(source_index),data,placement,Visibility::default()));
        }
    }

    let (mut trimesh, mut convex) = (0, 0);
    for collision in level.entities.iter().filter_map(|e| e.collision.as_ref()) {
        match collision.kind {
            level::CollisionShape::Trimesh => trimesh += 1,
            level::CollisionShape::Convex => convex += 1,
        }
    }
    info!(
        "{solids} solids drawn, {points} point entities marked, \
         collision: {trimesh} trimesh + {convex} convex (Avian)"
    );
}

/// One material per texture. The level file carries nothing about how shiny a
/// surface is, and a level built out of brushes is mostly not.
///
/// Brush UVs run well past 0..1, so the sampler has to repeat; Bevy's default
/// clamps and would smear the edge pixel across a wall. The 64 pixel textures
/// these maps use are drawn to be seen as pixels, hence nearest.
fn material(image: Option<Handle<Image>>) -> StandardMaterial {
    let Some(image) = image else {
        // No file matched, or the texture resolved to a Godot `.tres`, which
        // holds no image anyone else can read.
        return StandardMaterial {
            base_color: Color::srgb(0.55, 0.55, 0.58),
            perceptual_roughness: 0.9,
            ..default()
        };
    };
    // `maps` may name a normal map next to this one, which spawn_level adds.
    // Roughness and the other maps are left to the game.
    StandardMaterial {
        base_color_texture: Some(image),
        perceptual_roughness: 0.9,
        ..default()
    }
}

/// Brush UVs run well past 0..1, so the sampler has to repeat; Bevy's default
/// clamps and would smear the edge pixel along a wall. The 64 pixel textures
/// these maps use are drawn to be seen as pixels, hence nearest.
fn load_image(assets: &AssetServer, path: &str, paint: bool, srgb: bool) -> Handle<Image> {
    assets
        .load_builder()
        .with_settings(move |settings: &mut ImageLoaderSettings| {
            settings.is_srgb = srgb;
            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: if paint { ImageAddressMode::ClampToEdge } else { ImageAddressMode::Repeat },
                address_mode_v: if paint { ImageAddressMode::ClampToEdge } else { ImageAddressMode::Repeat },
                mag_filter: ImageFilterMode::Nearest,
                min_filter: ImageFilterMode::Linear,
                mipmap_filter: ImageFilterMode::Linear,
                ..default()
            });
        })
        .load(path.to_owned())
}

fn translucent(image: Option<Handle<Image>>) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(0.55, 0.8, 1.0, 0.55),
        base_color_texture: image,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 0.15,
        ..default()
    }
}

fn sun(entity: &level::Entity) -> DirectionalLight {
    let colour = entity.colour("light_color").unwrap_or([1.0, 1.0, 1.0, 1.0]);
    DirectionalLight {
        color: Color::srgb(colour[0], colour[1], colour[2]),
        // Godot's energy is a multiplier and Bevy wants lux, so this is a
        // choice, not a conversion: full daylight scaled by what the mapper
        // asked for.
        illuminance: entity.float("light_energy", 1.0) * light_consts::lux::FULL_DAYLIGHT,
        shadow_maps_enabled: entity.float("shadow_enabled", 1.0) != 0.0,
        ..default()
    }
}

/// A stable hue per classname, so two pickups look alike and a pickup and a
/// spawner do not.
fn classname_colour(classname: &str) -> Color {
    let mut hash: u32 = 2166136261;
    for byte in classname.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    Color::hsl((hash % 360) as f32, 0.75, 0.55)
}

fn spawn_camera(
    mut commands: Commands,
    level: Res<level::Level>,
    args: Res<Args>,
    sky: Res<Sky>,
    mesh_dir: Res<MeshDir>,
    assets: Res<AssetServer>,
) {
    let start = level
        .entities
        .iter()
        .find(|entity| entity.classname == "info_player_start");
    let (eye, yaw) = match start {
        Some(entity) => (
            Vec3::from(entity.transform.translation) + Vec3::Y * 1.6,
            entity.transform.yaw(),
        ),
        None => (Vec3::new(0.0, 8.0, 16.0), 0.0),
    };

    let transform = match args.at {
        Some(at) => match args.look {
            Some(look) => Transform::from_translation(at).looking_to(look, Vec3::Y),
            None => Transform::from_translation(at).looking_at(Vec3::ZERO, Vec3::Y),
        },
        None => Transform::from_translation(eye).with_rotation(Quat::from_euler(
            EulerRot::YXZ,
            yaw,
            -0.15,
            0.0,
        )),
    };

    let mut camera = commands.spawn((Camera3d::default(), transform));
    if let Some(sky) = &sky.0 {
        let cube = assets.load(mesh_dir.0.join(&sky.cubemap).to_string_lossy().into_owned());
        camera.insert(sky.camera(cube));
        if let Some(fog) = sky.fog() {
            camera.insert(fog);
        }
    }
    let has_collision = level
        .entities
        .iter()
        .any(|entity| entity.collision.as_ref().is_some_and(|c| c.parts > 0));
    if args.fly || start.is_none() || !has_collision {
        camera.insert(FreeCamera {
            walk_speed: 6.0,
            run_speed: 18.0,
            ..default()
        });
    }
    if let Some(sky) = sky.0.as_ref().filter(|_| !level.entities.iter().any(|e|e.classname == "light_sun")) {
        commands.spawn((
            Name::new("sun"),
            sky.sun(),
            CascadeShadowConfigBuilder { num_cascades: 4, maximum_distance: 100.0, first_cascade_far_bound: 8.0, ..default() }.build(),
        ));
    }
}

fn spawn_hud(mut commands: Commands, level: Res<level::Level>) {
    let font = TextFont {
        font_size: FontSize::Px(13.0),
        ..default()
    };

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        children![(
            Text::new(format!(
                "{}\n{} entities, {} textures\n\n\
                 hold right mouse  look around\n\
                 WASD  walk    space  jump    shift  faster    --fly  free camera\n\
                 wheel  speed    P  markers    C  collision    F12  screenshot\n\nAvian physics: solid collisions and sensors. Game-specific door / portal / enemy logic is separate.",
                level.name,
                level.entities.len(),
                level.textures.len()
            )),
            font.clone(),
            TextColor(Color::WHITE),
        )],
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
        children![(Readout, Text::new(""), font, TextColor(Color::WHITE))],
    ));
}

/// The camera's position in metres and in the map units the mapper reads off
/// editor's grid, which is what makes this useful for checking a map
/// rather than only looking at it.
fn update_readout(
    camera: Single<&Transform, With<Camera3d>>,
    level: Res<level::Level>,
    mut readout: Single<&mut Text, With<Readout>>,
) {
    let metres = camera.translation;
    let units = Vec3::new(metres.z, metres.x, metres.y) * level.scale;
    readout.0 = format!(
        "{:.2} {:.2} {:.2} m     {:.0} {:.0} {:.0} map units",
        metres.x, metres.y, metres.z, units.x, units.y, units.z
    );
}

fn toggle_markers(
    input: Res<ButtonInput<KeyCode>>,
    mut markers: Query<&mut Visibility, With<Marker>>,
) {
    if !input.just_pressed(KeyCode::KeyP) {
        return;
    }
    for mut visibility in &mut markers {
        *visibility = match *visibility {
            Visibility::Hidden => Visibility::Inherited,
            _ => Visibility::Hidden,
        };
    }
}

/// C shows the collision geometry the export wrote, as a green wash over the
/// walls it lines up with.
fn toggle_collision(
    input: Res<ButtonInput<KeyCode>>,
    mut hulls: Query<&mut Visibility, With<CollisionView>>,
) {
    if !input.just_pressed(KeyCode::KeyC) {
        return;
    }
    for mut visibility in &mut hulls {
        *visibility = match *visibility {
            Visibility::Hidden => Visibility::Inherited,
            _ => Visibility::Hidden,
        };
    }
}

fn screenshot_on_key(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut taken: Local<u32>,
) {
    if !input.just_pressed(KeyCode::F12) {
        return;
    }
    let path = format!("screenshot-{}.png", *taken);
    *taken += 1;
    info!("writing {path}");
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

/// `--shot`: draw a while so the meshes and textures are in, write one frame,
/// wait for it to reach the disk, quit.
fn auto_screenshot(
    mut commands: Commands,
    args: Res<Args>,
    capturing: Query<(), With<Capturing>>,
    mut exit: MessageWriter<AppExit>,
    mut frame: Local<u32>,
    mut shot_at: Local<u32>,
    assets: Res<AssetServer>,
    meshes: Query<&Mesh3d>,
    loaded_meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
) {
    if args.physics_check {
        return;
    }
    let Some(path) = &args.shot else {
        return;
    };
    *frame += 1;
    if *shot_at == 0 {
        if *frame >= args.frames.max(1) {
            let ready = meshes
                .iter()
                .all(|mesh| loaded_meshes.contains(mesh.0.id()))
                && materials.iter().all(|(_, mat)| {
                    mat.base_color_texture
                        .as_ref()
                        .is_none_or(|image| assets.is_loaded_with_dependencies(image.id()))
                });
            if !ready {
                if *frame > args.frames + 1800 {
                    error!("Meshes or textures did not load; refusing a misleading screenshot");
                    exit.write(AppExit::error());
                }
                return;
            }
            info!("All mesh and material image dependencies loaded before screenshot");
            info!("writing {}", path.display());
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path.clone()));
            *shot_at = *frame;
        }
    } else if capturing.is_empty() && *frame > *shot_at + 5 {
        exit.write(AppExit::Success);
    }
}

fn attach_animations(mut commands:Commands,level:Res<level::Level>,roots:Query<(Entity,&SourceEntity),Added<SourceEntity>>) {
    for (entity,source) in &roots {
        let e=&level.entities[source.0];
        commands.entity(entity).insert(heron_animation::HeronAnimationName(e.name.clone()));
        if let Some(data)=e.properties.get("_heron_trigger").and_then(level::Value::as_str) {
            if let Ok(trigger)=heron_animation::HeronAnimationTrigger::from_json(data){commands.entity(entity).insert(trigger);}
        }
        if let Some(data)=level.entities[source.0].properties.get("_heron_animation").and_then(level::Value::as_str) {
            match heron_animation::HeronAnimation::from_json(data) {
                Ok(animation)=>{commands.entity(entity).insert(animation);},
                Err(why)=>warn!("Animation: {why}"),
            }
        }
    }
}

fn prepare_water_surfaces(mut commands:Commands,mut meshes:ResMut<Assets<Mesh>>,children:Query<(Entity,&ChildOf,&Mesh3d),Without<heron_water_render::HeronWaterSurface>>,roots:Query<&heron_water::HeronWater>,cameras:Query<Entity,(With<Camera3d>,Without<heron_water_render::HeronWaterCamera>)>){
    for camera in &cameras {commands.entity(camera).insert(heron_water_render::HeronWaterCamera::default());}
    for (entity,parent,handle) in &children {let Ok(water)=roots.get(parent.parent())else{continue};if !water.0.surface_material{continue;}let Some(source)=meshes.get(&handle.0)else{continue};let Some(bevy::mesh::VertexAttributeValues::Float32x3(rest))=source.attribute(Mesh::ATTRIBUTE_POSITION)else{continue};let rest=rest.clone();let upward=match source.attribute(Mesh::ATTRIBUTE_NORMAL){Some(bevy::mesh::VertexAttributeValues::Float32x3(normals))=>normals.iter().map(|n|n[1]>0.5).collect(),_=>vec![true;rest.len()]};let copy=source.clone();commands.entity(entity).insert((Mesh3d(meshes.add(copy)),heron_water_render::HeronWaterSurface{data:water.0.clone(),rest,upward}));}
}

/// The feature lab's GLBs through Bevy's own glTF loader, without a window or GPU.
/// Export with examples/engine-checks/export_feature_lab.py first.
#[cfg(test)]
mod feature_materials {
    use bevy::gltf::{Gltf, GltfMaterial, GltfMesh};
    use bevy::image::ImageSampler;
    use bevy::render::settings::{RenderCreation, WgpuSettings};
    use bevy::render::RenderPlugin;
    use bevy::window::ExitCondition;
    use bevy::prelude::*;

    fn load(app: &mut App, file: &str) -> Handle<Gltf> {
        let handle: Handle<Gltf> = app.world().resource::<AssetServer>().load(file.to_owned());
        for _ in 0..2000 {
            app.update();
            let server = app.world().resource::<AssetServer>();
            if server.is_loaded_with_dependencies(handle.id()) {
                break;
            }
            assert!(!server.load_state(handle.id()).is_failed(), "{file} failed to load");
        }
        handle
    }

    fn gltf<'a>(app: &'a App, handle: &Handle<Gltf>) -> &'a Gltf {
        app.world().resource::<Assets<Gltf>>().get(handle).expect("loaded")
    }

    fn material(app: &App, handle: &Handle<Gltf>, name: &str) -> GltfMaterial {
        let handle = gltf(app, handle).named_materials.get(name).expect("the material is named");
        app.world().resource::<Assets<GltfMaterial>>().get(handle).expect("a material").clone()
    }

    fn wrap(app: &App, image: &Option<Handle<Image>>) -> Option<bevy::image::ImageAddressMode> {
        let image = app.world().resource::<Assets<Image>>().get(image.as_ref()?)?;
        match &image.sampler {
            ImageSampler::Descriptor(d) => Some(d.address_mode_u),
            ImageSampler::Default => None,
        }
    }

    #[test]
    #[ignore = "export the feature lab with examples/engine-checks/export_feature_lab.py bevy first"]
    fn exported_feature_lab_glbs_keep_their_materials_in_bevy() {
        let directory = std::env::var("HERON_BEVY_FEATURE").unwrap_or_else(|_| "/tmp/heron-feature-bevy".into());
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<bevy::winit::WinitPlugin>()
                .set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, ..default() })
                .set(AssetPlugin { file_path: directory, ..default() })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(WgpuSettings { backends: None, ..default() })),
                    ..default()
                }),
        );
        app.finish();
        app.cleanup();

        let normal = load(&mut app, "feature_lab/func_detail_4.glb");
        let brick = material(&app, &normal, "brick_wall");
        let mesh = app.world().resource::<Assets<GltfMesh>>().get(&gltf(&app, &normal).meshes[0]).expect("a mesh").clone();
        let primitive = app.world().resource::<Assets<Mesh>>().get(&mesh.primitives[0].mesh).expect("a primitive");
        let tangents = primitive.attribute(Mesh::ATTRIBUTE_TANGENT).is_some();
        let normal_map = brick
            .normal_map_texture
            .as_ref()
            .and_then(|h| app.world().resource::<Assets<Image>>().get(h))
            .map(|i| (i.width(), i.height(), i.texture_descriptor.format.is_srgb()));
        println!("normal map: {normal_map:?}, tangents {tangents}");
        assert_eq!(normal_map, Some((512, 512, false)));
        assert!(tangents);

        let surface = load(&mut app, "feature_lab/func_detail_3.glb");
        let glowing = material(&app, &surface, "bbpaint_747068d3d47ef28b");
        println!("emissive {:?} with a map {}", glowing.emissive, glowing.emissive_texture.is_some());
        assert!((glowing.emissive.red - 3.0).abs() < 1e-4 && glowing.emissive_texture.is_some());

        let colours = load(&mut app, "feature_lab/func_detail_1.glb");
        let clamped = gltf(&app, &colours)
            .named_materials
            .keys()
            .map(|name| wrap(&app, &material(&app, &colours, name).base_color_texture))
            .collect::<Vec<_>>();
        println!("paint wrapping {clamped:?}");
        assert!(clamped.iter().all(|w| *w == Some(bevy::image::ImageAddressMode::ClampToEdge)));

        let model = load(&mut app, "feature_lab/heron_model.glb");
        let names: Vec<_> = gltf(&app, &model).named_materials.keys().cloned().collect();
        for name in &names {
            let m = material(&app, &model, name);
            assert!(m.normal_map_texture.is_some() && m.metallic_roughness_texture.is_some());
            assert!(m.occlusion_texture.is_some() && m.emissive_texture.is_some());
        }
        println!("model: {} materials with normal, metal-roughness, occlusion and emissive maps", names.len());
        println!("FEATURE_LAB Bevy glTF PASS");
    }
}
