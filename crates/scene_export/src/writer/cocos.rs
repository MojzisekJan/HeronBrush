//! A level file and a loader component for Cocos Creator 3.8.
//!
//! Cocos Creator 3 reads glTF through its asset pipeline and its world is right-handed
//! with Y up and -Z forward, in metres -- which is the frame the level is already in, so
//! nothing here converts anything. That is worth saying plainly, because every other
//! engine target has a conversion in it and the absence of one looks like an omission.
//!
//! So the Cocos target writes the neutral level JSON the three.js target writes, the
//! editor's sky as six images, and one TypeScript component that reads the level and
//! builds it at runtime: models, colliders, animation, triggers, sky and sun. A
//! component rather than a `.scene`: a Cocos scene is the editor's own serialized form,
//! keyed by asset uuids the importer assigns, and a file written from outside it would
//! name uuids that do not exist yet. The component is the same for every level; the
//! level file says everything that differs.

use super::sky::Sky;
use crate::level::{CollisionMesh, Level};

/// One file the Cocos writer asks to be written, path relative to the export directory.
#[derive(Debug, Clone)]
pub struct CocosFile {
    /// Where it goes, relative to the export directory.
    pub path: String,
    /// What is in it.
    pub contents: Vec<u8>,
}

/// How the Cocos export is written.
#[derive(Debug, Clone, Default)]
pub struct CocosOptions {
    /// The sky, sun and fog to light the scene with, if any.
    pub sky: Option<Sky>,
}

/// The side of the sky's cube faces, in pixels.
const SKY_SIZE: u32 = 256;
/// The faces in the order `TextureCube.fromTexture2DArray` takes them.
const FACES: [&str; 6] = ["right", "left", "top", "bottom", "front", "back"];

/// Writes the level JSON, the sky's faces and the loader component.
pub fn write(level: &Level, collision: &[CollisionMesh], options: &CocosOptions) -> Vec<CocosFile> {
    let mut json: serde_json::Value = serde_json::from_str(&crate::writer::three::write(level, collision)).expect("level JSON");
    for (entity, value) in level.entities.iter().zip(json["entities"].as_array_mut().unwrap()) {
        let [x, y, z] = entity.transform.rotation_yxz;
        let q = glam::DQuat::from_euler(glam::EulerRot::YXZ, y, x, z);
        value["transform"]["rotation_quaternion"] = serde_json::json!(q.to_array());
    }
    let mut files = Vec::new();
    if let Some(sky) = &options.sky {
        // Cocos samples the cube with the view direction as it is, in the layout
        // WebGL and Direct3D share.
        let faces = sky.png_faces(SKY_SIZE, super::sky::cube_direction);
        let mut names = Vec::new();
        for (face, png) in FACES.iter().zip(faces) {
            let path = format!("{}/sky_{face}.png", level.name);
            names.push(path.trim_end_matches(".png").to_string());
            files.push(CocosFile { path, contents: png });
        }
        let to_sun = sky.to_sun();
        json["environment"] = serde_json::json!({
            "faces": names, "to_sun": to_sun, "sun_color": sky.sun_color, "sun_intensity": sky.sun_intensity,
            "ambient_intensity": sky.ambient_intensity, "sky": sky.sky, "ground": sky.ground,
            "fog": sky.fog.map(|(color, density)| serde_json::json!({"color": color, "density": density})),
        });
    }
    files.push(CocosFile { path: format!("{}.json", level.name), contents: serde_json::to_string_pretty(&json).expect("level JSON").into_bytes() });
    files.push(CocosFile { path: "HeronBrushLevel.ts".to_string(), contents: include_str!("cocos_runtime.ts").as_bytes().to_vec() });
    files.push(CocosFile { path: "COCOS-README.md".to_string(), contents: include_str!("cocos_readme.md").as_bytes().to_vec() });
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{Entity, Level, Transform};

    fn level() -> Level {
        Level {
            format: 2,
            generator: "test".to_string(),
            name: "arena".to_string(),
            scale: 32.0,
            textures: Vec::new(),
            entities: vec![Entity {
                name: "worldspawn".to_string(),
                classname: "worldspawn".to_string(),
                mesh: Some("arena/worldspawn.glb".to_string()),
                transform: Transform::default(),
                ..Entity::default()
            }],
        }
    }

    #[test]
    fn it_writes_the_level_and_a_loader() {
        let files = write(&level(), &[], &CocosOptions::default());
        let names: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(names, ["arena.json", "HeronBrushLevel.ts", "COCOS-README.md"]);
    }

    #[test]
    fn the_level_file_retains_neutral_fields_and_preserves_yxz_rotation() {
        let mut level = level();
        level.entities[0].transform.rotation_yxz = [0.37, -1.12, 0.64];
        let files = write(&level, &[], &CocosOptions::default());
        let mut json: serde_json::Value = serde_json::from_slice(&files[0].contents).unwrap();
        let q = json["entities"][0]["transform"].as_object_mut().unwrap().remove("rotation_quaternion").unwrap();
        let q = glam::DQuat::from_xyzw(q[0].as_f64().unwrap(), q[1].as_f64().unwrap(), q[2].as_f64().unwrap(), q[3].as_f64().unwrap());
        let basis = crate::scene::basis_from_euler_yxz(crate::math::Vec3::from_array(level.entities[0].transform.rotation_yxz));
        let point = crate::math::Vec3::new(2., 3., 5.);
        let expected = crate::math::Vec3::new(basis[0] * 2. + basis[1] * 3. + basis[2] * 5., basis[3] * 2. + basis[4] * 3. + basis[5] * 5., basis[6] * 2. + basis[7] * 3. + basis[8] * 5.);
        assert!((q * point - expected).length() < 1e-12);
        let neutral: serde_json::Value = serde_json::from_str(&crate::writer::three::write(&level, &[])).unwrap();
        assert_eq!(json, neutral);
    }

    #[test]
    fn nothing_is_converted_on_the_way_in() {
        // Cocos shares the level's own frame, so a loader that negated or swapped an
        // axis would be wrong. The generated one does none.
        let files = write(&level(), &[], &CocosOptions::default());
        let loader = String::from_utf8(files[1].contents.clone()).unwrap();
        assert!(loader.contains("node.setPosition(t.translation[0], t.translation[1], t.translation[2])"), "{loader}");
        assert!(!loader.contains("-t.translation"), "{loader}");
    }

    #[test]
    fn a_sky_becomes_six_faces_and_the_levels_environment() {
        let sky = Sky { sky: [0.1, 0.4, 0.85], horizon: [0.7, 0.8, 0.9], ground: [0.2, 0.18, 0.15], sun_color: [1., 0.9, 0.8], sun_intensity: 3., ambient_intensity: 0.5, sun_elevation: 45., sun_azimuth: 0., exposure: 1., fog: Some(([0.5, 0.6, 0.7], 0.02)) };
        let files = write(&level(), &[], &CocosOptions { sky: Some(sky) });
        assert_eq!(files.iter().filter(|f| f.path.starts_with("arena/sky_") && f.contents.starts_with(b"\x89PNG")).count(), 6);
        let json: serde_json::Value = serde_json::from_slice(&files.iter().find(|f| f.path == "arena.json").unwrap().contents).unwrap();
        assert_eq!(json["environment"]["faces"][4], "arena/sky_front");
        assert_eq!(json["environment"]["fog"]["density"], serde_json::json!(0.02f32));
    }
}
