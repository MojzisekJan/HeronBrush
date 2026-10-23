//! A JSON level file, a runtime module and a playable HTML page for PlayCanvas.
//!
//! The level file is the Babylon.js one (see [`super::babylon`]): the three.js level
//! with the collision triangles and hulls folded in and a `light` block on each light
//! entity.
//!
//! PlayCanvas is right-handed with Y up and -Z forward, the level's own frame, so
//! nothing is converted. `heron_playcanvas.js` loads each `.glb` as a container asset,
//! gives each entity with collision rigidbody and collision components backed by
//! ammo.js (static, kinematic for movers, a trigger volume for sensors), plays
//! `_heron_animation`, wires `_heron_trigger` to it, and applies the water, lights,
//! sky and fog. The page embeds the meshes, the sky and the runtime, loads PlayCanvas
//! and ammo.js from jsDelivr, and walks the level with a capsule body of its own.

use super::sky::Sky;
use crate::level::{CollisionMesh, Level};
use serde_json::{json, Value};

/// The exact PlayCanvas engine build the page loads, from jsDelivr. ammo.js comes
/// from the engine's repository at the same tag.
pub const PLAYCANVAS_VERSION: &str = "2.23.1";

/// `heron_playcanvas.js`, the ES module a game imports to load the level.
pub const RUNTIME: &str = include_str!("playcanvas_runtime.js");

/// The sky panorama the page embeds is small: it lights and backs a playtest, and
/// the exported `sky.hdr` beside the level is the full-size one.
const EMBEDDED_SKY: (u32, u32) = (256, 128);

/// Renders `level` as the JSON described in the module docs.
///
/// `collision` is `LevelResult::collision`, aligned with `level.entities`, as for
/// [`super::three::write`].
pub fn write(level: &Level, collision: &[CollisionMesh]) -> String {
    super::babylon::write(level, collision)
}

/// A directly openable playtest page. The meshes, a small sky panorama and the
/// runtime are embedded; PlayCanvas and ammo.js load from jsDelivr.
pub fn write_html(
    level: &Level,
    collision: &[CollisionMesh],
    meshes: &[crate::level::MeshFile],
    sky: Option<&Sky>,
) -> String {
    let embedded: std::collections::BTreeMap<_, _> = level
        .entities
        .iter()
        .filter_map(|entity| {
            let path = entity.mesh.as_ref()?;
            let mesh = meshes
                .iter()
                .find(|mesh| path.rsplit('/').next() == Some(mesh.file_name.as_str()))?;
            Some((path.clone(), crate::encode_base64(&mesh.bytes)))
        })
        .collect();
    let mut level_json: Value =
        serde_json::from_str(&write(level, collision)).expect("serialized level");
    let mut panorama = Value::Null;
    if let Some(sky) = sky {
        level_json["environment"] = super::three::environment(sky, None);
        let (width, height) = EMBEDDED_SKY;
        panorama = json!(crate::encode_base64(&sky.panorama_hdr(
            width,
            height,
            super::sky::playcanvas_panorama_direction,
        )));
    }
    let data = json!({ "level": level_json, "meshes": embedded, "sky": panorama });
    // JSON lives in a raw-text script element, so map properties must not close it.
    let json = data.to_string().replace('<', "\\u003c");
    include_str!("playcanvas_player.html")
        .replace("__PLAYCANVAS_VERSION__", PLAYCANVAS_VERSION)
        .replace("__RUNTIME__", &super::babylon::inline_module(RUNTIME))
        .replace("__LEVEL_DATA__", &json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{Entity, Transform};

    fn level() -> Level {
        Level {
            format: 2,
            generator: "heron_brush".into(),
            name: "arena".into(),
            scale: 32.0,
            textures: vec![],
            entities: vec![
                Entity {
                    name: "lamp".into(),
                    classname: "light".into(),
                    properties: [
                        (
                            "light_color".into(),
                            crate::level::Value::Color([1., 0.5, 0.25, 1.]),
                        ),
                        ("light_energy".into(), crate::level::Value::Float(2.5)),
                    ]
                    .into(),
                    transform: Transform {
                        translation: [1., 2., 3.],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Entity {
                    name: "door".into(),
                    classname: "func_door".into(),
                    solid: true,
                    mesh: Some("arena/door.glb".into()),
                    ..Default::default()
                },
            ],
        }
    }

    fn sky() -> Sky {
        Sky {
            sky: [0.2, 0.4, 0.9],
            horizon: [0.7, 0.75, 0.8],
            ground: [0.3, 0.3, 0.3],
            sun_color: [1.0, 0.95, 0.9],
            sun_intensity: 3.0,
            ambient_intensity: 1.0,
            sun_elevation: 40.0,
            sun_azimuth: 30.0,
            exposure: 1.0,
            fog: None,
        }
    }

    #[test]
    fn the_level_is_babylon_s_with_its_light_blocks() {
        let v: Value = serde_json::from_str(&write(&level(), &[])).expect("level JSON");
        assert_eq!(v["entities"][0]["light"]["kind"], "point");
        assert_eq!(v["entities"][0]["light"]["energy"], 2.5);
        assert!(v["entities"][1].get("light").is_none());
        assert_eq!(v["entities"][1]["mesh"], "arena/door.glb");
        assert_eq!(
            write(&level(), &[]),
            super::super::babylon::write(&level(), &[])
        );
    }

    #[test]
    fn the_page_embeds_meshes_sky_and_runtime_and_escapes_map_text() {
        let mut level = level();
        level.name = "</script><script>alert(1)</script>".into();
        let meshes = [crate::level::MeshFile {
            file_name: "door.glb".into(),
            bytes: vec![0, 1, 2, 255],
        }];
        let html = write_html(&level, &[], &meshes, Some(&sky()));
        let data = html
            .split("id=\"level-data\" type=\"application/json\">")
            .nth(1)
            .expect("the data element")
            .split("</script>")
            .next()
            .expect("its end");
        assert!(!data.contains('<'));
        let parsed: Value = serde_json::from_str(data).expect("JSON data");
        assert_eq!(parsed["level"]["name"], level.name);
        assert_eq!(
            parsed["meshes"]["arena/door.glb"],
            crate::encode_base64(&meshes[0].bytes)
        );
        let sky = data_encoding::BASE64
            .decode(parsed["sky"].as_str().expect("an embedded sky").as_bytes())
            .expect("base64");
        assert!(sky.starts_with(b"#?RADIANCE"));
        assert!(parsed["level"]["environment"]["to_sun"].is_array());
        // The runtime is inlined as plain declarations, and the versions pinned: the
        // engine from npm, ammo.js from the engine's repository at the same tag.
        assert!(html.contains("class HeronAnimation "));
        assert!(!html.lines().any(|line| line.starts_with("export ")));
        assert!(html.contains(&format!(
            "npm/playcanvas@{PLAYCANVAS_VERSION}/build/playcanvas.min.js"
        )));
        assert!(html.contains(&format!(
            "gh/playcanvas/engine@v{PLAYCANVAS_VERSION}/examples/assets/wasm/ammo/"
        )));
        assert!(!html.contains("__"), "a placeholder was left in the page");
    }

    #[test]
    fn the_playtest_tests_triggers_along_the_whole_player() {
        // A trigger brush can start above the floor, as the feature lab's does: the
        // player's feet alone would stand under it and never fire it.
        let html = write_html(&level(), &[], &[], None);
        assert!(html.contains("trigger.update(dt, body, animations)"));
        for point in ["feet", "middle", "head"] {
            assert!(html.contains(&format!("['player', {point}]")), "{point}");
        }
    }

    #[test]
    fn the_runtime_module_exports_what_a_game_uses() {
        for name in [
            "export async function loadLevel",
            "export class HeronAnimation ",
            "export class HeronAnimationTrigger",
            "export class HeronWater",
        ] {
            assert!(RUNTIME.contains(name), "{name}");
        }
        // Movers get kinematic bodies, sensors a collision component alone.
        assert!(RUNTIME.contains("record.animation ? 'kinematic' : 'static'"));
        assert!(RUNTIME.contains("collision.sensor ? null"));
    }
}
