//! A flat-text level file for raylib and other C loaders.
//!
//! `heron_brush export` writes `<name>.heron_brush.json` by default, which needs a JSON parser
//! on the other side. raylib is a C library and pulling in a JSON parser to
//! read one small file is more than the job needs. This writes the same
//! [`Level`] as line-oriented text a `while (fgets(...))` loop can walk:
//!
//! ```text
//! format 2
//! name arena
//! scale 32
//!
//! texture wall textures/wall.png 64 64
//!
//! entity worldspawn worldspawn
//! solid 1
//! mesh arena/worldspawn.glb
//! pos 0 0 0
//! surface 0 wall
//! collision trimesh 1 1
//! ctri 16 0 -16  -16 0 -16  -16 -1 -16
//! end
//! ```
//!
//! The `.glb` meshes are byte for byte the ones the neutral path writes, so
//! raylib's own `LoadModel` reads them without help. Only the index file
//! changes format, plus one addition: raylib cannot get at the glTF collision
//! mesh, so this writes collision triangles (`ctri`), exact convex vertex
//! clouds (`chull` / `chv` / `chend`), and legacy fallback boxes (`cbox`) into the text from `LevelResult::collision`, and marks a trigger
//! volume with a trailing `sensor` token on the `collision` line. See
//! `docs/level-format.md` for every field; they mean the same as in the JSON.
//!
//! Each `chull N` starts a hull with N following `chv x y z` records and one
//! `chend` terminator. Vertex records have bounded line length regardless of
//! hull size. Points are entity-local metres; apply the entity transform.
//! Hull order matches `cbox` order. A loader supporting hull physics must use
//! each exact hull instead of its corresponding fallback box, never both.
//! Existing format-2 readers may ignore these additive keywords and use cbox.
//!
//! One keyword per line, whitespace separated. Blank lines and lines starting
//! with `#` are comments. An `entity` line opens a record and `end` closes it;
//! everything between belongs to that entity. Every token except a `prop`
//! string value is free of spaces and newlines.

use crate::level::{CollisionMesh, CollisionShape, Level, Value};

/// Optional baseline renderer for raylib's generated preview and project code.
/// The source entity transforms are baked into these static light records.
pub fn lighting(level: &Level, sky: Option<&super::sky::Sky>) -> String {
    let mut lights = Vec::new();
    for entity in &level.entities {
        if let Some(light) = super::common_entities::light(entity) {
            let t = &entity.transform;
            let direction = glam::DQuat::from_euler(
                glam::EulerRot::YXZ,
                t.rotation_yxz[1],
                t.rotation_yxz[0],
                t.rotation_yxz[2],
            ) * -glam::DVec3::Z;
            lights.push((
                light.kind,
                t.translation,
                direction.to_array(),
                light.color,
                light.energy,
                light.range,
                light.half_angle,
            ));
        }
    }
    if !lights.iter().any(|l| l.0 == "Directional") {
        if let Some(sky) = sky {
            lights.push((
                "Directional",
                [0.; 3],
                sky.to_sun().map(|v| -f64::from(v)),
                [
                    f64::from(sky.sun_color[0]),
                    f64::from(sky.sun_color[1]),
                    f64::from(sky.sun_color[2]),
                    1.,
                ],
                f64::from(sky.sun_intensity),
                1.,
                45.,
            ));
        }
    }
    let mut records = String::new();
    for (kind, position, direction, color, energy, range, angle) in &lights {
        let kind = match *kind {
            "Spot" => 1,
            "Directional" => 2,
            _ => 0,
        };
        let a = |v: &[f64]| {
            v.iter()
                .take(3)
                .map(|v| format!("{v:.9}f"))
                .collect::<Vec<_>>()
                .join(",")
        };
        records.push_str(&format!(
            "{{{kind},{{{}}},{{{}}},{{{}}},{energy:.9}f,{range:.9}f,{:.9}f}},\n",
            a(position),
            a(direction),
            a(color),
            angle.to_radians().cos()
        ));
    }
    if records.is_empty() {
        records.push_str("{0},\n");
    }
    let data = format!(
        "#define HB_RENDER_LIGHT_COUNT {}\nstatic const HbRenderLight hb_render_lights[]={{\n{records}}};",
        lights.len()
    );
    include_str!("raylib_lighting.h")
        .replace("__LIGHT_DATA__", &data)
        .replace("__LIGHT_CAPACITY__", &lights.len().max(1).to_string())
}

/// Renders a [`Level`] as the flat text described in the module docs.
///
/// `collision` is `LevelResult::collision`, aligned with `level.entities`.
/// Pass `&[]` to leave collision geometry out; then a `collision` line still
/// says the shape's kind, but no inline collision records follow.
pub fn write(level: &Level, collision: &[CollisionMesh]) -> String {
    let mut out = String::new();

    out.push_str("# heron_brush level, flat text for raylib and other C loaders.\n");
    out.push_str("# One keyword per line. '#' and blank lines are comments.\n");
    push_kv(&mut out, "format", &level.format.to_string());
    push_kv(&mut out, "generator", &level.generator);
    push_kv(&mut out, "name", &level.name);
    push_kv(&mut out, "scale", &num(level.scale));

    out.push('\n');
    for texture in &level.textures {
        let image = texture.image.as_deref().unwrap_or("-");
        out.push_str(&format!(
            "texture {} {} {} {}\n",
            texture.name,
            image,
            num(texture.size[0]),
            num(texture.size[1]),
        ));
    }

    for (index, entity) in level.entities.iter().enumerate() {
        out.push('\n');
        out.push_str(&format!("entity {} {}\n", entity.classname, entity.name));

        for group in &entity.group {
            out.push_str(&format!("group {group}\n"));
        }
        out.push_str(&format!("solid {}\n", u8::from(entity.solid)));
        if let Some(mesh) = &entity.mesh {
            out.push_str(&format!("mesh {mesh}\n"));
        }

        let t = &entity.transform;
        out.push_str(&format!(
            "pos {} {} {}\n",
            num(t.translation[0]),
            num(t.translation[1]),
            num(t.translation[2]),
        ));
        out.push_str(&format!(
            "rot {} {} {}\n",
            num(t.rotation_yxz[0]),
            num(t.rotation_yxz[1]),
            num(t.rotation_yxz[2]),
        ));
        out.push_str(&format!(
            "scale {} {} {}\n",
            num(t.scale[0]),
            num(t.scale[1]),
            num(t.scale[2]),
        ));

        for surface in &entity.surfaces {
            out.push_str(&format!(
                "surface {} {}\n",
                surface.primitive, surface.texture
            ));
        }
        if let Some(c) = &entity.collision {
            let kind = match c.kind {
                CollisionShape::Trimesh => "trimesh",
                CollisionShape::Convex => "convex",
            };
            let sensor = if c.sensor { " sensor" } else { "" };
            out.push_str(&format!(
                "collision {kind} {} {}{sensor}\n",
                c.mesh, c.parts
            ));

            // Entity-local metres, with full entity transform applied by the
            // consumer. Exact hulls accompany the legacy AABB fallback.
            if let Some(cm) = collision.get(index) {
                for tri in cm.triangles.as_chunks::<3>().0 {
                    out.push_str("ctri");
                    for point in tri {
                        for v in point {
                            out.push(' ');
                            out.push_str(&num(*v as f64));
                        }
                    }
                    out.push('\n');
                }
                for hull in &cm.hulls {
                    out.push_str(&format!("chull {}\n", hull.len()));
                    for point in hull {
                        out.push_str(&format!(
                            "chv {} {} {}\n",
                            num(point[0] as f64),
                            num(point[1] as f64),
                            num(point[2] as f64),
                        ));
                    }
                    out.push_str("chend\n");
                }
                for [lo, hi] in &cm.boxes {
                    out.push_str(&format!(
                        "cbox {} {} {} {} {} {}\n",
                        num(lo[0] as f64),
                        num(lo[1] as f64),
                        num(lo[2] as f64),
                        num(hi[0] as f64),
                        num(hi[1] as f64),
                        num(hi[2] as f64),
                    ));
                }
            }
        }
        if let Some(scene) = &entity.scene {
            out.push_str(&format!("scene {scene}\n"));
        }
        for target in &entity.targets {
            out.push_str(&format!("target {target}\n"));
        }
        for tag in &entity.tags {
            out.push_str(&format!("tag {tag}\n"));
        }
        for (key, value) in &entity.properties {
            push_prop(&mut out, key, value);
        }

        out.push_str("end\n");
    }

    out
}

fn push_kv(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    // An empty value, such as a string property the mapper left blank, writes
    // the keyword alone. The reader treats a missing value as "".
    if !value.is_empty() {
        out.push(' ');
        out.push_str(value);
    }
    out.push('\n');
}

fn push_prop(out: &mut String, key: &str, value: &Value) {
    match value {
        Value::Bool(b) => push_kv(out, &format!("prop bool {key}"), if *b { "1" } else { "0" }),
        Value::Int(i) => push_kv(out, &format!("prop int {key}"), &i.to_string()),
        Value::Float(f) => push_kv(out, &format!("prop float {key}"), &num(*f)),
        // A string value runs to the end of the line, so it may hold spaces.
        // Newlines cannot survive a line-oriented format; flatten them.
        Value::String(s) => push_kv(
            out,
            &format!("prop string {key}"),
            &s.replace(['\n', '\r'], " "),
        ),
        Value::Vec2([x, y]) => push_kv(
            out,
            &format!("prop vec2 {key}"),
            &format!("{} {}", num(*x), num(*y)),
        ),
        Value::Vec3([x, y, z]) => push_kv(
            out,
            &format!("prop vec3 {key}"),
            &format!("{} {} {}", num(*x), num(*y), num(*z)),
        ),
        Value::Color([r, g, b, a]) => push_kv(
            out,
            &format!("prop color {key}"),
            &format!("{} {} {} {}", num(*r), num(*g), num(*b), num(*a)),
        ),
        // The compact text format flattens array scalars; the JSON sidecar
        // retains the complete typed structure for richer game data.
        Value::Array(items) => {
            let joined: Vec<String> = items.iter().map(scalar).collect();
            push_kv(out, &format!("prop array {key}"), &joined.join(" "))
        }
    }
}

/// One scalar value as a bare token, for array items.
fn scalar(value: &Value) -> String {
    match value {
        Value::Bool(b) => (if *b { "1" } else { "0" }).to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => num(*f),
        Value::String(s) => s.replace([' ', '\n', '\r'], "_"),
        _ => "?".to_string(),
    }
}

/// A number with no trailing zeros and no exponent for the everyday range,
/// falling back to Rust's shortest round-tripping form.
fn num(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{Collision, Entity, Surface, Texture, Transform};

    fn sample() -> Level {
        Level {
            format: 2,
            generator: "heron_brush 0.1.0".into(),
            name: "arena".into(),
            scale: 32.0,
            textures: vec![
                Texture {
                    surface: None,
                    name: "wall".into(),
                    image: Some("textures/wall.png".into()),
                    maps: vec![],
                    size: [64.0, 64.0],
                    clamp: false,
                },
                Texture {
                    surface: None,
                    name: "sky".into(),
                    image: None,
                    maps: vec![],
                    size: [0.0, 0.0],
                    clamp: false,
                },
            ],
            entities: vec![Entity {
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
                    mesh: 1,
                    parts: 1,
                    sensor: false,
                }),
                transform: Transform::default(),
                ..Default::default()
            }],
        }
    }

    #[test]
    fn the_header_comes_first() {
        let text = write(&sample(), &[]);
        let lines: Vec<&str> = text.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(lines[0], "format 2");
        assert!(text.contains("\nformat 2\n"));
        assert!(text.contains("\nname arena\n"));
        assert!(text.contains("\nscale 32\n"));
    }

    #[test]
    fn a_texture_with_no_image_writes_a_dash() {
        let text = write(&sample(), &[]);
        assert!(text.contains("texture wall textures/wall.png 64 64\n"));
        assert!(text.contains("texture sky - 0 0\n"));
    }

    #[test]
    fn an_entity_record_opens_and_closes() {
        let text = write(&sample(), &[]);
        assert!(text.contains("entity worldspawn worldspawn\n"));
        assert!(text.contains("\nsolid 1\n"));
        assert!(text.contains("\nmesh arena/worldspawn.glb\n"));
        assert!(text.contains("\nsurface 0 wall\n"));
        assert!(text.contains("\ncollision trimesh 1 1\n"));
        assert!(text.trim_end().ends_with("end"));
    }

    #[test]
    fn properties_carry_their_type() {
        let mut level = sample();
        let props = &mut level.entities[0].properties;
        props.insert("speed".into(), Value::Float(200.0));
        props.insert("count".into(), Value::Int(3));
        props.insert("title".into(), Value::String("front door".into()));
        props.insert("tint".into(), Value::Color([1.0, 0.5, 0.0, 1.0]));
        let text = write(&level, &[]);
        assert!(text.contains("prop float speed 200\n"));
        assert!(text.contains("prop int count 3\n"));
        assert!(text.contains("prop string title front door\n"));
        assert!(text.contains("prop color tint 1 0.5 0 1\n"));
    }
    #[test]
    fn exact_convex_hulls_survive_without_long_lines_or_box_substitution() {
        let mut level = sample();
        let entity = &mut level.entities[0];
        entity.collision.as_mut().unwrap().kind = CollisionShape::Convex;
        entity.collision.as_mut().unwrap().sensor = true;
        entity.transform.rotation_yxz = [0.25, 0.5, 0.75];
        entity.transform.scale = [2.0, 3.0, 4.0];
        entity.targets.push("custom_target".into());
        entity
            .properties
            .insert("custom_speed".into(), Value::Float(2.5));
        // Deliberately asymmetric, with enough vertices to exceed a typical
        // fgets buffer if the entire hull were written on one line.
        let hull: Vec<[f32; 3]> = (0..400).map(|i| [i as f32 * 0.125, -0.75, 1.25]).collect();
        let wedge = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 3.0],
        ];
        let cm = CollisionMesh {
            hulls: vec![hull.clone(), wedge.clone()],
            boxes: vec![
                [[0.0, -0.75, 0.0], [50.0, 1.0, 3.0]],
                [[0.0; 3], [2.0, 1.0, 3.0]],
            ],
            ..Default::default()
        };
        let text = write(&level, &[cm]);
        let mut restored = Vec::new();
        let mut expected = 0;
        for line in text.lines() {
            let parts: Vec<_> = line.split_whitespace().collect();
            match parts.first().copied() {
                Some("chull") => {
                    expected = parts[1].parse::<usize>().unwrap();
                    restored.push(Vec::<[f32; 3]>::new());
                }
                Some("chv") => {
                    assert!(line.len() < 128);
                    restored.last_mut().unwrap().push([
                        parts[1].parse().unwrap(),
                        parts[2].parse().unwrap(),
                        parts[3].parse().unwrap(),
                    ]);
                }
                Some("chend") => assert_eq!(restored.last().unwrap().len(), expected),
                _ => {}
            }
        }
        assert_eq!(restored, vec![hull, wedge]);
        assert_eq!(text.lines().filter(|s| s.starts_with("cbox ")).count(), 2);
        assert!(text.contains("collision convex 1 1 sensor\n"));
        assert!(text.contains("rot 0.25 0.5 0.75\nscale 2 3 4\n"));
        assert!(text.contains("target custom_target\n"));
        assert!(text.contains("prop float custom_speed 2.5\n"));
    }
}
