//! A JSON level file and a playable HTML page for Three.js.
//!
//! Three.js reads `.glb` with its own `GLTFLoader` and has no physics, so the
//! neutral `<name>.heron_brush.json` is nearly right already: this writes the same
//! object with the collision geometry folded in as a flat triangle array per
//! entity, so a loader can build a raycast mesh or feed a physics library
//! without opening a `.glb`.
//!
//! ```json
//! {
//!   "format": 2,
//!   "name": "arena",
//!   "scale": 32,
//!   "textures": [{ "name": "wall", "image": "textures/wall.png", "size": [64, 64] }],
//!   // Baked face paint adds "clamp": true: draw it clamped, not repeated.
//!   "entities": [
//!     {
//!       "name": "worldspawn",
//!       "classname": "worldspawn",
//!       "transform": { "translation": [0,0,0], "rotation_yxz": [0,0,0], "scale": [1,1,1] },
//!       "mesh": "arena/worldspawn.glb",
//!       "surfaces": [{ "primitive": 0, "texture": "wall" }],
//!       "collision": {
//!         "kind": "trimesh",
//!         "triangles": [16,0,-16, -16,0,-16, -16,-1,-16, ...]
//!       }
//!     }
//!   ]
//! }
//! ```
//!
//! Three.js is Y-up, right-handed, `-Z` forward, the same as the Godot axes
//! heron_brush already uses, so nothing here converts coordinates. `rotation_yxz` is
//! Euler radians in Y, then X, then Z order (`new THREE.Euler(x, y, z,
//! 'YXZ')`). `triangles` is entity-local metres, nine numbers a triangle; add
//! the entity's `transform.translation` for world space. The `.glb` meshes
//! hold no collision, so `GLTFLoader` draws only what is visible.

use super::sky::Sky;
use crate::level::{CollisionMesh, Level};
use serde_json::{Value, json};

/// Renders `level` as the JSON described in the module docs.
///
/// `collision` is `LevelResult::collision`, aligned with `level.entities`;
/// pass `&[]` to leave the collision arrays out (a `collision` object then
/// still names the shape's kind, with no `triangles`).
pub fn write(level: &Level, collision: &[CollisionMesh]) -> String {
    let mut root = serde_json::to_value(level).unwrap_or(Value::Null);

    if let Some(entities) = root.get_mut("entities").and_then(Value::as_array_mut) {
        for (index, entity) in entities.iter_mut().enumerate() {
            if let Some(light) = super::common_entities::light(&level.entities[index]) {
                entity["light"] = json!({"kind": light.kind, "color": light.color, "energy": light.energy, "range": light.range, "half_angle": light.half_angle, "shadows": light.shadows});
            }
            let Some(mesh) = collision.get(index) else {
                continue;
            };
            if mesh.triangles.is_empty() && mesh.hulls.is_empty() {
                continue;
            }
            let Some(obj) = entity.as_object_mut() else {
                continue;
            };

            let slot = obj
                .entry("collision")
                .or_insert_with(|| json!({ "kind": "trimesh", "mesh": 0, "parts": 1 }));
            let Some(col) = slot.as_object_mut() else {
                continue;
            };

            if !mesh.triangles.is_empty() {
                let flat: Vec<f32> = mesh.triangles.iter().flat_map(|p| *p).collect();
                col.insert("triangles".to_string(), json!(flat));
            }
            if !mesh.hulls.is_empty() {
                let hulls: Vec<Vec<f32>> = mesh
                    .hulls
                    .iter()
                    .map(|hull| hull.iter().flat_map(|p| *p).collect())
                    .collect();
                col.insert("hulls".to_string(), json!(hulls));
            }
        }
    }

    serde_json::to_string_pretty(&root).unwrap_or_default()
}

/// A directly openable FPS playtest page. Meshes and albedo images are embedded;
/// the pinned Three.js runtime is loaded from a CDN.
/// `textures` maps material names to image data URLs.
// The HTML already supplies albedo images by material name. Do not embed a second
// copy in every preview GLB. Standalone GLB files keep their original complete assets.
fn preview_mesh(
    bytes: &[u8],
    albedos: &std::collections::BTreeMap<String, String>,
) -> Option<Vec<u8>> {
    fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
        Some(u32::from_le_bytes(
            bytes.get(offset..offset + 4)?.try_into().ok()?,
        ))
    }
    fn texture_indices(value: &Value, used: &mut std::collections::BTreeSet<usize>) {
        if let Some(map) = value.as_object() {
            for (key, item) in map {
                if key.ends_with("Texture") {
                    if let Some(index) = item.get("index").and_then(Value::as_u64) {
                        used.insert(index as usize);
                    }
                } else {
                    texture_indices(item, used);
                }
            }
        }
    }
    fn remap_textures(value: &mut Value, indices: &[Option<usize>]) -> Option<()> {
        if let Some(map) = value.as_object_mut() {
            for (key, item) in map {
                if key.ends_with("Texture") {
                    if let Some(index) = item.get("index").and_then(Value::as_u64) {
                        item["index"] = json!(indices.get(index as usize)?.as_ref()?);
                    }
                } else {
                    remap_textures(item, indices)?;
                }
            }
        }
        Some(())
    }
    if bytes.get(..4)? != b"glTF" || u32_at(bytes, 4)? != 2 {
        return None;
    }
    let length = u32_at(bytes, 12)? as usize;
    let mut doc: Value = serde_json::from_slice(bytes.get(20..20 + length)?).ok()?;
    let bin_offset = 20 + length;
    let bin_length = u32_at(bytes, bin_offset)? as usize;
    let binary = bytes.get(bin_offset + 8..bin_offset + 8 + bin_length)?;
    let materials = doc.get_mut("materials")?.as_array_mut()?;
    let mut changed = false;
    for material in materials.iter_mut() {
        if !albedos.contains_key(material.get("name").and_then(Value::as_str).unwrap_or("")) {
            continue;
        }
        let base = material
            .get_mut("pbrMetallicRoughness")?
            .as_object_mut()?
            .remove("baseColorTexture");
        if let Some(base) = base {
            changed = true;
            if material.get("emissiveTexture").and_then(|t| t.get("index")) == base.get("index") {
                material.as_object_mut()?.remove("emissiveTexture");
            }
        }
    }
    if !changed {
        return None;
    }
    let mut used = std::collections::BTreeSet::new();
    for material in materials.iter() {
        texture_indices(material, &mut used);
    }
    let old_textures = doc.get("textures")?.as_array()?.clone();
    let old_images = doc.get("images")?.as_array()?.clone();
    let mut texture_map = vec![None; old_textures.len()];
    let mut image_map = vec![None; old_images.len()];
    let mut textures = Vec::new();
    let mut images = Vec::new();
    for index in used {
        let mut texture = old_textures.get(index)?.clone();
        let source = texture.get("source")?.as_u64()? as usize;
        let target = if let Some(target) = *image_map.get(source)? {
            target
        } else {
            let target = images.len();
            images.push(old_images.get(source)?.clone());
            image_map[source] = Some(target);
            target
        };
        texture["source"] = json!(target);
        texture_map[index] = Some(textures.len());
        textures.push(texture);
    }
    for material in doc.get_mut("materials")?.as_array_mut()? {
        remap_textures(material, &texture_map)?;
    }
    let old_views = doc.get("bufferViews")?.as_array()?.clone();
    let mut removed = std::collections::BTreeSet::new();
    for (index, image) in old_images.iter().enumerate() {
        if image_map[index].is_none() {
            removed.insert(image.get("bufferView")?.as_u64()? as usize);
        }
    }
    let mut view_map = vec![None; old_views.len()];
    let mut views = Vec::new();
    let mut bin = Vec::new();
    for (index, view) in old_views.iter().enumerate() {
        if removed.contains(&index) {
            continue;
        }
        if view.get("buffer")?.as_u64()? != 0 {
            return None;
        }
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let offset = view.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let length = view.get("byteLength")?.as_u64()? as usize;
        let mut view = view.clone();
        view["byteOffset"] = json!(bin.len());
        bin.extend_from_slice(binary.get(offset..offset.checked_add(length)?)?);
        view_map[index] = Some(views.len());
        views.push(view);
    }
    fn remap_view(value: &mut Value, indices: &[Option<usize>]) -> Option<()> {
        if let Some(index) = value.get("bufferView").and_then(Value::as_u64) {
            value["bufferView"] = json!(indices.get(index as usize)?.as_ref()?);
        }
        Some(())
    }
    for accessor in doc.get_mut("accessors")?.as_array_mut()? {
        remap_view(accessor, &view_map)?;
        if let Some(sparse) = accessor.get_mut("sparse") {
            remap_view(sparse.get_mut("indices")?, &view_map)?;
            remap_view(sparse.get_mut("values")?, &view_map)?;
        }
    }
    for image in &mut images {
        remap_view(image, &view_map)?;
    }
    doc["images"] = json!(images);
    doc["textures"] = json!(textures);
    doc["bufferViews"] = json!(views);
    doc["buffers"][0]["byteLength"] = json!(bin.len());
    let mut json = serde_json::to_vec(&doc).ok()?;
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let mut out = Vec::new();
    for word in [
        0x46546c67u32,
        2,
        (28 + json.len() + bin.len()) as u32,
        json.len() as u32,
        0x4e4f534a,
    ] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend(json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004e4942u32.to_le_bytes());
    out.extend(bin);
    Some(out)
}

/// Write the playable preview, embedding geometry and shared albedo data once.
/// The sky, sun and fog a loader lights the level with, as the level file's
/// `environment`. Colours are linear and directions in the level's frame; `panorama`
/// names the sky as an equirectangular `.hdr`, when one is written beside the level.
pub fn environment(sky: &Sky, panorama: Option<&str>) -> Value {
    let mut block = json!({
        "to_sun": sky.to_sun(), "sun_color": sky.sun_color, "sun_intensity": sky.sun_intensity,
        "ambient_intensity": sky.ambient_intensity, "sky": sky.sky, "horizon": sky.horizon,
        "ground": sky.ground, "exposure": sky.exposure,
        "fog": sky.fog.map(|(color, density)| json!({"color": color, "density": density})),
    });
    if let Some(panorama) = panorama {
        block["panorama"] = json!(panorama);
    }
    block
}

pub fn write_html(
    level: &Level,
    collision: &[CollisionMesh],
    meshes: &[crate::level::MeshFile],
    textures: &std::collections::BTreeMap<String, String>,
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
            let preview = preview_mesh(&mesh.bytes, textures);
            Some((
                path.clone(),
                crate::encode_base64(preview.as_deref().unwrap_or(&mesh.bytes)),
            ))
        })
        .collect();
    let mut level_json =
        serde_json::from_str::<Value>(&write(level, collision)).expect("serialized level");
    // The page draws the sky from these few numbers rather than carrying an image, so
    // a playtest's live updates stay small.
    if let Some(sky) = sky {
        level_json["environment"] = environment(sky, None);
    }
    let data = json!({
        "level": level_json,
        "meshes": embedded,
        "textures": textures,
    });
    // JSON lives in a raw-text script element, so map properties must not close it.
    let json = data.to_string().replace('<', "\\u003c");
    include_str!("three_player.html")
        .replace(
            "__ANIMATION_RUNTIME__",
            &include_str!("three_animation.js").replace("export class", "class"),
        )
        .replace(
            "__COLLISION_RUNTIME__",
            &include_str!("three_collision.js").replace("export function", "function"),
        )
        .replace(
            "__WATER_RUNTIME__",
            &include_str!("three_water.js").replace("export class", "class"),
        )
        .replace(
            "__LIGHT_RUNTIME__",
            &include_str!("three_lights.js").replace("export function", "function"),
        )
        .replace("__LEVEL_DATA__", &json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{Collision, CollisionShape, Entity, Surface, Texture, Transform};

    fn level() -> Level {
        Level {
            format: 2,
            generator: "heron_brush".into(),
            name: "arena".into(),
            scale: 32.0,
            textures: vec![Texture {
                surface: None,
                name: "wall".into(),
                image: Some("textures/wall.png".into()),
                maps: vec![],
                size: [64.0, 64.0],
                clamp: false,
            }],
            entities: vec![
                Entity {
                    name: "worldspawn".into(),
                    classname: "worldspawn".into(),
                    solid: true,
                    mesh: Some("arena/worldspawn.glb".into()),
                    surfaces: vec![Surface {
                        primitive: 0,
                        texture: "wall".into(),
                    }],
                    collision: Some(Collision {
                        kind: CollisionShape::Trimesh,
                        mesh: 0,
                        parts: 1,
                        sensor: false,
                    }),
                    transform: Transform::default(),
                    ..Default::default()
                },
                Entity {
                    name: "enemy0".into(),
                    classname: "fps_enemy".into(),
                    solid: false,
                    transform: Transform {
                        translation: [1.0, 2.0, 3.0],
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ],
        }
    }

    fn cube() -> CollisionMesh {
        // one triangle is enough for the test
        CollisionMesh {
            triangles: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            ..Default::default()
        }
    }

    #[test]
    fn html_embeds_meshes_and_escapes_map_properties() {
        let mut level = level();
        level.name = "</script><script>alert(1)</script>".into();
        let meshes = [crate::level::MeshFile {
            file_name: "worldspawn.glb".into(),
            bytes: vec![0, 1, 2, 255],
        }];
        let html = write_html(&level, &[cube()], &meshes, &Default::default(), None);
        let data = html
            .split("id=\"level-data\" type=\"application/json\">")
            .nth(1)
            .unwrap()
            .split("</script>")
            .next()
            .unwrap();
        assert!(!data.contains('<'));
        let parsed: Value = serde_json::from_str(data).unwrap();
        assert_eq!(parsed["level"]["name"], level.name);
        assert_eq!(
            data_encoding::BASE64
                .decode(
                    parsed["meshes"]["arena/worldspawn.glb"]
                        .as_str()
                        .unwrap()
                        .as_bytes()
                )
                .unwrap(),
            meshes[0].bytes
        );
        assert_eq!(
            parsed["level"]["entities"][0]["collision"]["triangles"]
                .as_array()
                .unwrap()
                .len(),
            9
        );
    }

    #[test]
    fn the_playtest_tests_triggers_along_the_whole_player() {
        // Found running the feature lab's page: its trigger brush starts 0.25 m above
        // the floor, and the page tested only the player's feet, so standing in the
        // trigger never opened the door.
        let html = write_html(&level(), &[], &[], &Default::default(), None);
        let call = html
            .split("trigger.update(dt, ")
            .nth(1)
            .expect("the page updates triggers");
        assert!(call.starts_with("body,"), "{}", &call[..40]);
        assert!(html.contains("['player', capsule.start.clone().lerp(capsule.end, 0.5)]"));
        assert!(html.contains(
            "['player', capsule.end.clone().addScaledVector(THREE.Object3D.DEFAULT_UP, radius)]"
        ));
    }

    #[test]
    fn it_is_valid_json_with_the_level_fields() {
        let text = write(&level(), &[]);
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["name"], "arena");
        assert_eq!(v["scale"], 32.0);
        assert_eq!(v["entities"][0]["classname"], "worldspawn");
        assert_eq!(v["entities"][0]["mesh"], "arena/worldspawn.glb");
    }

    #[test]
    fn collision_triangles_are_folded_in_flat() {
        let cm = [cube(), CollisionMesh::default()];
        let v: Value = serde_json::from_str(&write(&level(), &cm)).unwrap();
        let tris = v["entities"][0]["collision"]["triangles"]
            .as_array()
            .unwrap();
        assert_eq!(tris.len(), 9);
        assert_eq!(tris[3], 1.0);
        assert_eq!(v["entities"][0]["collision"]["kind"], "trimesh");
        // A point entity with no collision keeps none.
        assert!(v["entities"][1].get("collision").is_none());
    }

    #[test]
    fn a_sensor_flag_survives() {
        let mut l = level();
        l.entities[0].collision.as_mut().unwrap().sensor = true;
        l.entities[0].classname = "trigger_multiple".into();
        let cm = [cube(), CollisionMesh::default()];
        let v: Value = serde_json::from_str(&write(&l, &cm)).unwrap();
        assert_eq!(v["entities"][0]["collision"]["sensor"], true);
    }
}

#[cfg(test)]
mod preview_tests {
    use super::*;
    #[test]
    fn removing_shared_albedo_keeps_normal_image_and_geometry_views_valid() {
        let doc = json!({"materials":[{"name":"stone","normalTexture":{"index":1},"pbrMetallicRoughness":{"baseColorTexture":{"index":0}},"emissiveTexture":{"index":0}}],
            "textures":[{"source":0},{"source":1}],"images":[{"bufferView":1},{"bufferView":2}],
            "buffers":[{"byteLength":12}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":4},{"buffer":0,"byteOffset":4,"byteLength":4},{"buffer":0,"byteOffset":8,"byteLength":4}],
            "accessors":[{"bufferView":0}]});
        let mut json = serde_json::to_vec(&doc).unwrap();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let mut input = Vec::new();
        for word in [
            0x46546c67u32,
            2,
            (40 + json.len()) as u32,
            json.len() as u32,
            0x4e4f534a,
        ] {
            input.extend_from_slice(&word.to_le_bytes());
        }
        input.extend(json);
        input.extend_from_slice(&12u32.to_le_bytes());
        input.extend_from_slice(&0x004e4942u32.to_le_bytes());
        input.extend_from_slice(b"GEOMBASE NRM");
        let output = preview_mesh(
            &input,
            &std::collections::BTreeMap::from([(
                "stone".into(),
                "data:image/png;base64,unused".into(),
            )]),
        )
        .unwrap();
        let length = u32::from_le_bytes(output[12..16].try_into().unwrap()) as usize;
        let compact: Value = serde_json::from_slice(&output[20..20 + length]).unwrap();
        assert_eq!(compact["images"].as_array().unwrap().len(), 1);
        assert_eq!(compact["images"][0]["bufferView"], 1);
        assert_eq!(compact["materials"][0]["normalTexture"]["index"], 0);
        assert!(
            compact["materials"][0]["pbrMetallicRoughness"]
                .get("baseColorTexture")
                .is_none()
        );
        assert_eq!(&output[28 + length..32 + length], b"GEOM");
        assert_eq!(&output[32 + length..36 + length], b" NRM");
    }
}
