//! A Defold collection for `heron_brush export --engine defold`.
//!
//! The neutral path writes `<name>.heron_brush.json` plus a `.glb` per solid entity.
//! Defold cannot read that JSON, and its model component draws every mesh in a
//! `.glb`, so this writes a set of Defold source files instead:
//!
//! ```text
//! arena.collection            the level: one game object per entity
//! arena/worldspawn.model      a model component pointing at worldspawn.glb
//! arena/worldspawn.collisionobject   static collision, one hull per brush
//! arena/enemy0.script         a point entity's keys as go.property lines
//! ```
//!
//! The `.glb` files are the same ones the neutral path writes, minus the
//! collision mesh (Defold would draw it). Collision comes from
//! [`CollisionMesh`] as `TYPE_HULL` shapes, which Defold's 3D physics reads
//! directly. The map and Defold share an axis convention once Quake's
//! Z-up is folded to Y-up, which is exactly what heron_brush already did, so nothing
//! here converts coordinates.
//!
//! Everything is Defold protobuf text. The collection's game objects are
//! embedded instances whose `data` field is the escaped text of a prototype.

use super::sky::Sky;
use crate::level::{CollisionMesh, Level, Value};

fn text_property<'a>(entity: &'a crate::level::Entity, key: &str) -> Option<&'a str> {
    match entity.properties.get(key) {
        Some(Value::String(value)) => Some(value),
        _ => None,
    }
}
fn number_property(entity: &crate::level::Entity, key: &str, default: f64) -> f64 {
    match entity.properties.get(key) {
        Some(Value::Float(value)) => *value,
        Some(Value::Int(value)) => *value as f64,
        Some(Value::String(value)) => value.parse().unwrap_or(f64::NAN),
        None => default,
        _ => f64::NAN,
    }
}
fn engine_hint(key: &str) -> bool {
    key.starts_with("defold_sampler_")
        || key.starts_with("defold_component_")
        || key.starts_with("defold_property_")
        || matches!(
            key,
            "defold_go"
                | "defold_body_type"
                | "defold_mass"
                | "defold_friction"
                | "defold_restitution"
                | "defold_group"
                | "defold_mask"
                | "defold_linear_damping"
                | "defold_angular_damping"
                | "defold_locked_rotation"
                | "defold_material"
                | "defold_sampler"
        )
}

fn property_override(
    key: &str,
    value: &Value,
    entity: &crate::level::Entity,
) -> Result<Option<(String, String, String, String)>, String> {
    if key.starts_with("defold_property_type_") {
        return Ok(None);
    }
    let Some(rest) = key.strip_prefix("defold_property_") else {
        return Ok(None);
    };
    let Some((component, property)) = rest.split_once("__") else {
        return Err(format!("{key} needs component__property"));
    };
    if component.is_empty()
        || property.is_empty()
        || !rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!("invalid override name {key}"));
    }
    let (mut kind, text) = match value {
        Value::Bool(v) => ("BOOLEAN", v.to_string()),
        Value::Int(v) => ("NUMBER", v.to_string()),
        Value::Float(v) if v.is_finite() => ("NUMBER", f(*v)),
        Value::String(v) => ("HASH", v.clone()),
        Value::Vec3(v) => (
            "VECTOR3",
            v.iter().map(|v| f(*v)).collect::<Vec<_>>().join(", "),
        ),
        Value::Color(v) => (
            "VECTOR4",
            v.iter().map(|v| f(*v)).collect::<Vec<_>>().join(", "),
        ),
        Value::Array(values) if values.len() == 3 || values.len() == 4 => {
            let values = values
                .iter()
                .map(|v| match v {
                    Value::Int(v) => Ok(*v as f64),
                    Value::Float(v) if v.is_finite() => Ok(*v),
                    _ => Err(format!("{key} vector needs numeric values")),
                })
                .collect::<Result<Vec<_>, _>>()?;
            (
                if values.len() == 3 {
                    "VECTOR3"
                } else {
                    "VECTOR4"
                },
                values.iter().map(|v| f(*v)).collect::<Vec<_>>().join(", "),
            )
        }
        _ => return Err(format!("unsupported native property type for {key}")),
    };
    if let Some(hint) = text_property(entity, &format!("defold_property_type_{rest}")) {
        kind = match (hint, kind) {
            ("number", "NUMBER") => "NUMBER",
            ("boolean", "BOOLEAN") => "BOOLEAN",
            ("hash", "HASH") => "HASH",
            ("url", "HASH") => "URL",
            ("vector3", "VECTOR3") => "VECTOR3",
            ("vector4", "VECTOR4") => "VECTOR4",
            ("quat", "VECTOR4") => "QUAT",
            _ => {
                return Err(format!(
                    "{key}: property type {hint} is incompatible with the supplied value"
                ));
            }
        };
    }
    Ok(Some((component.into(), property.into(), kind.into(), text)))
}

/// Validate project-owned engine hints before an export writes any files.
pub fn validate(level: &Level) -> Result<(), String> {
    for entity in &level.entities {
        let fail = |message: &str| format!("Defold entity {}: {message}", entity.name);
        if let Some(value) = entity.properties.get("defold_go") {
            let Value::String(path) = value else {
                return Err(fail("defold_go must be a resource path"));
            };
            if !path.starts_with('/') || !path.ends_with(".go") || path.contains("..") {
                return Err(fail("defold_go must name a project .go resource"));
            }
            if entity.solid || entity.properties.contains_key("_heron_animation") {
                return Err(fail(
                    "defold_go replaces point entities; brush geometry and generated animations need embedded components instead",
                ));
            }
        }
        if let Some(value) = entity.properties.get("defold_body_type") {
            let Value::String(kind) = value else {
                return Err(fail("defold_body_type must be a string"));
            };
            if !matches!(
                kind.as_str(),
                "static" | "kinematic" | "dynamic" | "trigger" | "none"
            ) {
                return Err(fail(
                    "defold_body_type must be static, kinematic, dynamic, trigger or none",
                ));
            }
            if kind == "dynamic" && entity.properties.contains_key("_heron_animation") {
                return Err(fail(
                    "a dynamic body cannot also be driven by exported animation",
                ));
            }
        }
        for (key, default, maximum) in [
            ("defold_mass", 0., f64::INFINITY),
            ("defold_friction", 0.1, f64::INFINITY),
            ("defold_restitution", 0.5, 1.),
            ("defold_linear_damping", 0., 1.),
            ("defold_angular_damping", 0., 1.),
        ] {
            let value = number_property(entity, key, default);
            if !value.is_finite() || value < 0. || value > maximum {
                return Err(fail(&format!("invalid {key}")));
            }
        }
        if text_property(entity, "defold_body_type") == Some("dynamic")
            && number_property(entity, "defold_mass", 0.) <= 0.
        {
            return Err(fail("a dynamic body requires positive defold_mass"));
        }
        if text_property(entity, "defold_body_type") != Some("dynamic")
            && number_property(entity, "defold_mass", 0.) != 0.
        {
            return Err(fail("only dynamic bodies may have nonzero defold_mass"));
        }
        if let Some(value) = entity.properties.get("defold_locked_rotation") {
            if !matches!(value, Value::Bool(_))
                && !matches!(value,Value::String(s) if matches!(s.as_str(),"true"|"false"|"0"|"1"))
            {
                return Err(fail("defold_locked_rotation must be a boolean"));
            }
        }
        for key in [
            "defold_group",
            "defold_mask",
            "defold_material",
            "defold_sampler",
        ] {
            if let Some(value) = entity.properties.get(key) {
                let Value::String(value) = value else {
                    return Err(fail(&format!("{key} must be a string")));
                };
                if key != "defold_mask" && value.is_empty() {
                    return Err(fail(&format!("{key} cannot be empty")));
                }
                if key == "defold_material" && (!value.starts_with('/') || value.contains("..")) {
                    return Err(fail("defold_material must be a project resource path"));
                }
            }
        }
        let mut samplers = std::collections::HashSet::new();
        if let Some(name) = text_property(entity, "defold_sampler") {
            samplers.insert(name);
        }
        for (key, value) in &entity.properties {
            if let Some(slot) = key.strip_prefix("defold_sampler_") {
                if !matches!(
                    slot,
                    "normal" | "mr" | "ao" | "emissive" | "roughness" | "metallic"
                ) {
                    return Err(fail(&format!("unsupported material map slot {slot}")));
                }
                let Value::String(name) = value else {
                    return Err(fail(&format!("{key} must be a string")));
                };
                if name.is_empty()
                    || !name.chars().enumerate().all(|(i, c)| {
                        c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                    })
                {
                    return Err(fail(&format!("{key} must be a shader sampler identifier")));
                }
                if !samplers.insert(name) {
                    return Err(fail(&format!("duplicate material sampler {name}")));
                }
            }
        }
        for (key, value) in &entity.properties {
            if let Some(rest) = key.strip_prefix("defold_property_type_") {
                if !matches!(value, Value::String(_))
                    || !entity
                        .properties
                        .contains_key(&format!("defold_property_{rest}"))
                {
                    return Err(fail(
                        "property type hint needs a matching property override",
                    ));
                }
            }
            if let Some((component, _, _, _)) =
                property_override(key, value, entity).map_err(|error| fail(&error))?
            {
                if text_property(entity, "defold_go").is_none()
                    && !entity
                        .properties
                        .contains_key(&format!("defold_component_{component}"))
                {
                    return Err(fail(&format!(
                        "property override refers to missing component {component}"
                    )));
                }
            }
            if let Some(id) = key.strip_prefix("defold_component_") {
                if id.is_empty()
                    || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    || matches!(id, "mesh" | "collision" | "properties")
                {
                    return Err(fail(
                        "custom component id must be unique and cannot be mesh, collision or properties",
                    ));
                }
                let Value::String(path) = value else {
                    return Err(fail("custom component resource must be a string"));
                };
                if !path.starts_with('/') || path.contains("..") || path.contains(['\n', '\r']) {
                    return Err(fail("custom component must use a project resource path"));
                }
            }
        }
    }
    Ok(())
}

/// Where the generated files sit inside the Defold project, so the collection
/// can point at them with `/`-rooted resource paths.
#[derive(Debug, Clone)]
pub struct DefoldOptions {
    /// Resource path of the folder holding the `.glb` / `.model` /
    /// `.collisionobject` / `.script` files. Leading `/`, no trailing `/`,
    /// e.g. `/main/arena`.
    pub asset_dir: String,
    /// Joined onto each texture's project-relative `image`. `/` turns
    /// `textures/wall.png` into `/textures/wall.png`.
    pub texture_prefix: String,
    /// Additional portable material maps, keyed by material name.
    pub material_maps: std::collections::BTreeMap<String, Vec<(String, String)>>,
    /// The material every model component gets.
    pub model_material: String,
    /// The sampler that material binds the colour texture to.
    pub model_sampler: String,
    /// The sky to draw round the camera, if any.
    pub sky: Option<Sky>,
}

impl Default for DefoldOptions {
    fn default() -> Self {
        DefoldOptions {
            asset_dir: "/main/level".to_string(),
            texture_prefix: "/".to_string(),
            material_maps: Default::default(),
            model_material: "/builtins/materials/model.material".to_string(),
            model_sampler: "tex0".to_string(),
            sky: None,
        }
    }
}

/// One file to write, its path relative to the export's out directory.
#[derive(Debug, Clone)]
pub struct DefoldFile {
    pub path: String,
    pub contents: String,
}

/// Explicit shader samplers must be unique and every bound surface needs the map.
pub fn validate_material_bindings(level: &Level, options: &DefoldOptions) -> Result<(), String> {
    for entity in &level.entities {
        let hints: Vec<_> = entity
            .properties
            .keys()
            .filter_map(|k| {
                k.strip_prefix("defold_sampler_")
                    .map(|slot| (slot, k.as_str()))
            })
            .collect();
        if hints.is_empty() {
            continue;
        }
        let mut samplers = std::collections::HashSet::new();
        samplers.insert(text_property(entity, "defold_sampler").unwrap_or(&options.model_sampler));
        for (slot, key) in hints {
            let sampler = text_property(entity, key)
                .ok_or_else(|| format!("{}: {key} must name a sampler", entity.name))?;
            if !samplers.insert(sampler) {
                return Err(format!(
                    "{}: duplicate material sampler {sampler}",
                    entity.name
                ));
            }
            for surface in &entity.surfaces {
                let texture = level.textures.iter().find(|t| t.name == surface.texture);
                let maps = options
                    .material_maps
                    .get(&surface.texture)
                    .map(Vec::as_slice)
                    .unwrap_or_else(|| texture.map(|t| t.maps.as_slice()).unwrap_or(&[]));
                if !maps.iter().any(|(name, _)| name == slot) {
                    return Err(format!(
                        "{}: material {} has no {slot} map for sampler {sampler}",
                        entity.name, surface.texture
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Renders `level` as a Defold collection and its component files.
///
/// `collision` is `LevelResult::collision`, aligned with `level.entities`;
/// pass `&[]` and no collision objects come out.
pub fn write(level: &Level, collision: &[CollisionMesh], opts: &DefoldOptions) -> Vec<DefoldFile> {
    let dir = opts.asset_dir.trim_end_matches('/');
    let name = ident(&level.name);
    let water_module = format!(
        "{}/{name}_water",
        dir.rsplit_once('/').map(|v| v.0).unwrap_or("")
    )
    .trim_start_matches('/')
    .replace('/', ".");

    let mut files: Vec<DefoldFile> = Vec::new();
    files.push(DefoldFile {
        path: format!("{name}_water.lua"),
        contents: include_str!("defold_water.lua").into(),
    });
    let parent = dir.rsplit_once('/').map(|v| v.0).unwrap_or("");
    let has_water = level
        .entities
        .iter()
        .any(|e| e.properties.contains_key("_heron_water"));
    if has_water || opts.sky.is_some() {
        // A baseline 3D renderer: the stock one draws neither the sky nor water.
        files.push(DefoldFile {
            path: format!("{name}.render_script"),
            contents: include_str!("defold.render_script").into(),
        });
        files.push(DefoldFile {
            path: format!("{name}.render"),
            contents: format!("script: \"{parent}/{name}.render_script\"\n"),
        });
    }
    if has_water {
        for (suffix, contents) in [
            ("vp", include_str!("defold_water.vp")),
            ("fp", include_str!("defold_water.fp")),
            ("gui_script", include_str!("defold_water_view.gui_script")),
        ] {
            files.push(DefoldFile {
                path: format!("{name}_water.{suffix}"),
                contents: contents.into(),
            });
        }
        let material = format!(
            "name: \"water\"\ntags: \"heron_water\"\nvertex_program: \"{parent}/{name}_water.vp\"\nfragment_program: \"{parent}/{name}_water.fp\"\nvertex_space: VERTEX_SPACE_LOCAL\nvertex_constants {{name: \"mtx_worldview\" type: CONSTANT_TYPE_WORLDVIEW}}\nvertex_constants {{name: \"mtx_proj\" type: CONSTANT_TYPE_PROJECTION}}\nvertex_constants {{name: \"water_params\" type: CONSTANT_TYPE_USER value {{x:0 y:.035 z:1.5 w:0}}}}\nfragment_constants {{name: \"water_tint\" type: CONSTANT_TYPE_USER value {{x:.05 y:.38 z:.48 w:.55}}}}\nsamplers {{name: \"tex0\" wrap_u: WRAP_MODE_REPEAT wrap_v: WRAP_MODE_REPEAT filter_min: FILTER_MODE_MIN_LINEAR filter_mag: FILTER_MODE_MAG_LINEAR}}\n"
        );
        files.push(DefoldFile {
            path: format!("{name}_water.material"),
            contents: material,
        });
    }
    let mut instances = String::new();
    if let Some(sky) = &opts.sky {
        files.extend(sky_files(&name, dir, parent, sky));
        let proto = format!("components {{\n  id: \"sky\"\n  component: \"{dir}/sky.model\"\n}}\n");
        instances.push_str(&format!(
            "embedded_instances {{\n  id: \"heron_sky\"\n  data: \"{}\"\n}}\n",
            escape(&proto)
        ));
    }
    let mut used: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    let ids: std::collections::HashMap<_, _> = level
        .entities
        .iter()
        .map(|e| (e.name.clone(), unique(&mut used, ident(&e.name))))
        .collect();
    for (index, entity) in level.entities.iter().enumerate() {
        let id = ids[&entity.name].clone();
        if let Some(path) = text_property(entity, "defold_go") {
            instances.push_str(&native_instance(&id, entity, path));
            continue;
        }

        // What components this game object carries.
        let mut components: Vec<(String, String)> = Vec::new();

        if !entity.surfaces.is_empty() {
            if let Some(mesh) = &entity.mesh {
                let file = format!("{name}/{id}.model");
                files.push(DefoldFile {
                    path: file.clone(),
                    contents: model_file(entity, level, mesh, opts, dir),
                });
                components.push(("mesh".into(), format!("{dir}/{id}.model")));
            }
        }

        if let Some(co) = collision_file(entity, collision.get(index)) {
            let file = format!("{name}/{id}.collisionobject");
            files.push(DefoldFile {
                path: file,
                contents: co,
            });
            components.push(("collision".into(), format!("{dir}/{id}.collisionobject")));
        }

        if let Some(script) = script_file(entity, &ids, &water_module) {
            let file = format!("{name}/{id}.script");
            files.push(DefoldFile {
                path: file,
                contents: script,
            });
            components.push(("properties".into(), format!("{dir}/{id}.script")));
        }

        for (key, value) in &entity.properties {
            if let (Some(id), Value::String(path)) = (key.strip_prefix("defold_component_"), value)
            {
                components.push((id.into(), path.clone()));
            }
        }

        instances.push_str(&embedded_instance(&id, entity, &components));
    }

    let mut collection = String::new();
    collection.push_str(&format!("name: \"{name}\"\n"));
    collection.push_str("scale_along_z: 0\n");
    collection.push_str(&instances);

    files.insert(
        0,
        DefoldFile {
            path: format!("{name}.collection"),
            contents: collection,
        },
    );
    let mut json = level.to_json();
    if let Some(sky) = &opts.sky {
        let mut value: serde_json::Value = serde_json::from_str(&json).expect("level JSON");
        value["environment"] = super::three::environment(sky, None);
        json = serde_json::to_string_pretty(&value).expect("level JSON");
    }
    files.push(DefoldFile {
        path: format!("{name}.json"),
        contents: json,
    });
    if level
        .entities
        .iter()
        .any(|e| e.properties.contains_key("_heron_animation"))
    {
        files.push(DefoldFile {path:format!("{name}_animation.md"),contents:"# Defold keyframe animations\n\nAnimation playback is included in each animated entity's generated #properties script. Animations start automatically only when start_on is enabled. Animated collision objects are kinematic; sensor volumes remain triggers.\n\nSend messages to the game object's #properties URL: heron_play_animation { clip = \"open\" }, heron_pause_animation, heron_resume_animation, heron_stop_animation, heron_seek_animation { time = 1.5 }. Clip names are strings. stop restores the rest pose; once holds its final pose. Exported func_button, trigger_once and trigger_multiple targets are wired in their generated scripts. Put the player collider in the player physics group with a default mask. Entry/contact activates the targets. Send heron_activate to a button or trigger properties URL for explicit interaction. Once-only behavior and wait cooldowns are included; named open/close clips toggle.\n\nExample: msg.post(\"/level/func_animate#properties\", \"heron_play_animation\", {clip=\"open\"}). Adjust the collection instance and object id to your scene. Re-export overwrites generated scripts; keep gameplay in separate components.\n".into()});
    }
    files
}

/// The side of the sky's cube faces, in pixels.
const SKY_SIZE: u32 = 256;
/// Defold's cubemap sides, in the order the faces are baked (+X, -X, +Y, -Y, +Z, -Z).
const SKY_FACES: [&str; 6] = ["right", "left", "top", "bottom", "front", "back"];

/// The sky's text resources: the cubemap, its material and shaders, the model that
/// draws it, and the sun and fog as a Lua table for game code.
fn sky_files(name: &str, dir: &str, parent: &str, sky: &Sky) -> Vec<DefoldFile> {
    let mut cubemap = String::new();
    for face in SKY_FACES {
        cubemap.push_str(&format!("{face}: \"{dir}/sky_{face}.png\"\n"));
    }
    let material = format!(
        "name: \"heron_sky\"\ntags: \"heron_sky\"\nvertex_program: \"{parent}/{name}_sky.vp\"\nfragment_program: \"{parent}/{name}_sky.fp\"\nvertex_space: VERTEX_SPACE_LOCAL\nvertex_constants {{name: \"mtx_view\" type: CONSTANT_TYPE_VIEW}}\nvertex_constants {{name: \"mtx_proj\" type: CONSTANT_TYPE_PROJECTION}}\nsamplers {{name: \"sky\" wrap_u: WRAP_MODE_CLAMP_TO_EDGE wrap_v: WRAP_MODE_CLAMP_TO_EDGE filter_min: FILTER_MODE_MIN_LINEAR filter_mag: FILTER_MODE_MAG_LINEAR}}\n"
    );
    let model = format!(
        "mesh: \"{dir}/sky.gltf\"\nmaterials {{\n  name: \"sky\"\n  material: \"{parent}/{name}_sky.material\"\n  textures {{\n    sampler: \"sky\"\n    texture: \"{dir}/sky.cubemap\"\n  }}\n}}\n"
    );
    let v = |c: [f32; 3]| format!("vmath.vector3({}, {}, {})", c[0], c[1], c[2]);
    let fog = sky
        .fog
        .map(|(color, density)| format!("{{color = {}, density = {density}}}", v(color)))
        .unwrap_or_else(|| "nil".into());
    let environment = format!(
        "-- The map's sun, ambient light and fog, in the level's frame. Colours are linear.\nreturn {{\n  to_sun = {},\n  sun_color = {},\n  sun_intensity = {},\n  ambient_intensity = {},\n  fog = {fog},\n}}\n",
        v(sky.to_sun()),
        v(sky.sun_color),
        sky.sun_intensity,
        sky.ambient_intensity
    );
    vec![
        DefoldFile {
            path: format!("{name}/sky.cubemap"),
            contents: cubemap,
        },
        DefoldFile {
            path: format!("{name}/sky.model"),
            contents: model,
        },
        DefoldFile {
            path: format!("{name}/sky.gltf"),
            contents: cube_gltf(),
        },
        DefoldFile {
            path: format!("{name}_sky.material"),
            contents: material,
        },
        DefoldFile {
            path: format!("{name}_sky.vp"),
            contents: include_str!("defold_sky.vp").into(),
        },
        DefoldFile {
            path: format!("{name}_sky.fp"),
            contents: include_str!("defold_sky.fp").into(),
        },
        DefoldFile {
            path: format!("{name}_environment.lua"),
            contents: environment,
        },
    ]
}

/// The sky's six faces as the view shows them: Defold's model material does no tone
/// mapping.
pub fn sky_binaries(level_name: &str, sky: &Sky) -> Vec<(String, Vec<u8>)> {
    let name = ident(level_name);
    let files: Vec<(String, Vec<u8>)> = SKY_FACES
        .iter()
        .zip(sky.display_faces(SKY_SIZE, super::sky::cube_direction))
        .map(|(face, png)| (format!("{name}/sky_{face}.png"), png))
        .collect();
    files
}

/// A cube round the origin as a `.gltf` with its buffer inline, positions only, one
/// material named `sky`.
fn cube_gltf() -> String {
    let mut bin = Vec::new();
    for i in 0..8u32 {
        for axis in 0..3 {
            bin.extend((if i >> axis & 1 == 1 { 1f32 } else { -1f32 }).to_le_bytes());
        }
    }
    // Two triangles per side; the sky is seen from inside and drawn without culling.
    let quads = [
        [0, 2, 6, 4],
        [1, 5, 7, 3],
        [0, 4, 5, 1],
        [2, 3, 7, 6],
        [0, 1, 3, 2],
        [4, 6, 7, 5],
    ];
    for q in quads {
        for i in [q[0], q[1], q[2], q[0], q[2], q[3]] {
            bin.extend((i as u16).to_le_bytes());
        }
    }
    let json = serde_json::json!({
        "asset": {"version": "2.0", "generator": "heron_brush"},
        "scene": 0, "scenes": [{"nodes": [0]}], "nodes": [{"mesh": 0, "name": "sky"}],
        "materials": [{"name": "sky"}],
        "meshes": [{"name": "sky", "primitives": [{"attributes": {"POSITION": 0}, "indices": 1, "material": 0}]}],
        "buffers": [{"byteLength": bin.len(), "uri": format!("data:application/octet-stream;base64,{}", crate::encode_base64(&bin))}],
        "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 96, "target": 34962}, {"buffer": 0, "byteOffset": 96, "byteLength": 72, "target": 34963}],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 8, "type": "VEC3", "min": [-1, -1, -1], "max": [1, 1, 1]},
            {"bufferView": 1, "componentType": 5123, "count": 36, "type": "SCALAR"},
        ],
    });
    serde_json::to_string_pretty(&json).expect("glTF JSON")
}

// ---------------------------------------------------------------------------
// Collection

fn embedded_instance(
    id: &str,
    entity: &crate::level::Entity,
    components: &[(String, String)],
) -> String {
    let mut proto = String::new();
    for (cid, path) in components {
        proto.push_str(&format!(
            "components {{\n  id: \"{}\"\n  component: \"{}\"\n}}\n",
            escape(cid),
            escape(path)
        ));
    }

    let t = &entity.transform;
    let q = quat_yxz(t.rotation_yxz);

    let mut out = String::new();
    out.push_str("embedded_instances {\n");
    out.push_str(&format!("  id: \"{id}\"\n"));
    out.push_str(&format!("  data: \"{}\"\n", escape(&proto)));
    out.push_str(&format!(
        "  position {{\n    x: {}\n    y: {}\n    z: {}\n  }}\n",
        f(t.translation[0]),
        f(t.translation[1]),
        f(t.translation[2]),
    ));
    out.push_str(&format!(
        "  rotation {{\n    x: {}\n    y: {}\n    z: {}\n    w: {}\n  }}\n",
        f(q[0]),
        f(q[1]),
        f(q[2]),
        f(q[3]),
    ));
    out.push_str(&format!(
        "  scale3 {{\n    x: {}\n    y: {}\n    z: {}\n  }}\n",
        f(t.scale[0]),
        f(t.scale[1]),
        f(t.scale[2])
    ));
    out.push_str(&component_overrides(entity));
    out.push_str("}\n");
    out
}

fn component_overrides(entity: &crate::level::Entity) -> String {
    let mut components: std::collections::BTreeMap<String, Vec<(String, String, String)>> =
        Default::default();
    for (key, value) in &entity.properties {
        if let Ok(Some((component, property, kind, value))) = property_override(key, value, entity)
        {
            components
                .entry(component)
                .or_default()
                .push((property, kind, value));
        }
    }
    let mut out = String::new();
    for (component, properties) in components {
        out.push_str(&format!(
            "  component_properties {{\n    id: \"{}\"\n",
            escape(&component)
        ));
        for (property, kind, value) in properties {
            out.push_str(&format!(
                "    properties {{ id: \"{}\" value: \"{}\" type: PROPERTY_TYPE_{kind} }}\n",
                escape(&property),
                escape(&value)
            ));
        }
        out.push_str("  }\n");
    }
    out
}

fn native_instance(id: &str, entity: &crate::level::Entity, path: &str) -> String {
    let t = &entity.transform;
    let q = quat_yxz(t.rotation_yxz);
    format!(
        "instances {{\n  id: \"{}\"\n  prototype: \"{}\"\n  position {{ x: {} y: {} z: {} }}\n  rotation {{ x: {} y: {} z: {} w: {} }}\n  scale3 {{ x: {} y: {} z: {} }}\n{}}}\n",
        escape(id),
        escape(path),
        f(t.translation[0]),
        f(t.translation[1]),
        f(t.translation[2]),
        f(q[0]),
        f(q[1]),
        f(q[2]),
        f(q[3]),
        f(t.scale[0]),
        f(t.scale[1]),
        f(t.scale[2]),
        component_overrides(entity)
    )
}

// ---------------------------------------------------------------------------
// Model

fn model_file(
    entity: &crate::level::Entity,
    level: &Level,
    mesh: &str,
    opts: &DefoldOptions,
    dir: &str,
) -> String {
    let water_material = format!(
        "{}/{}_water.material",
        dir.rsplit_once('/').map(|v| v.0).unwrap_or(""),
        ident(&level.name)
    );

    let glb = format!("{dir}/{}", mesh.rsplit('/').next().unwrap_or(mesh));
    let mut out = format!("mesh: \"{glb}\"\n");

    let mut seen: Vec<&str> = Vec::new();
    for surface in &entity.surfaces {
        if seen.contains(&surface.texture.as_str()) {
            continue;
        }
        seen.push(&surface.texture);

        out.push_str("materials {\n");
        out.push_str(&format!("  name: \"{}\"\n", surface.texture));
        out.push_str(&format!(
            "  material: \"{}\"\n",
            escape(
                text_property(entity, "defold_material").unwrap_or(
                    if entity
                        .properties
                        .get("_heron_water")
                        .and_then(|v| if let Value::String(s) = v {
                            serde_json::from_str::<serde_json::Value>(s).ok()
                        } else {
                            None
                        })
                        .is_some_and(|d| d["surface_material"] == true)
                    {
                        water_material.as_str()
                    } else {
                        &opts.model_material
                    }
                )
            )
        ));
        if let Some(image) = level
            .textures
            .iter()
            .find(|t| t.name == surface.texture)
            .and_then(|t| t.image.as_deref())
        {
            let path = join_resource(&opts.texture_prefix, image);
            out.push_str(&format!(
                "  textures {{\n    sampler: \"{}\"\n    texture: \"{path}\"\n  }}\n",
                escape(text_property(entity, "defold_sampler").unwrap_or(&opts.model_sampler))
            ));
        }
        let texture = level.textures.iter().find(|t| t.name == surface.texture);
        let maps = opts
            .material_maps
            .get(&surface.texture)
            .map(Vec::as_slice)
            .unwrap_or_else(|| texture.map(|t| t.maps.as_slice()).unwrap_or(&[]));
        let albedo_sampler = text_property(entity, "defold_sampler").unwrap_or(&opts.model_sampler);
        for (slot, path) in maps {
            if let Some(sampler) = text_property(entity, &format!("defold_sampler_{slot}")) {
                if sampler == albedo_sampler {
                    continue;
                }
                let path = join_resource(&opts.texture_prefix, path);
                out.push_str(&format!(
                    "  textures {{\n    sampler: \"{sampler}\"\n    texture: \"{}\"\n  }}\n",
                    escape(&path)
                ));
            }
        }
        out.push_str("}\n");
    }
    out
}

// ---------------------------------------------------------------------------
// Collision

fn collision_file(entity: &crate::level::Entity, mesh: Option<&CollisionMesh>) -> Option<String> {
    if text_property(entity, "defold_body_type") == Some("none") {
        return None;
    }
    entity.collision.as_ref()?;
    let mesh = mesh?;

    // Every hull becomes a TYPE_HULL shape; a concave brush with no hulls
    // falls back to the box around its triangles.
    let mut shapes = String::new();
    let mut data: Vec<f32> = Vec::new();
    let mut count = 0usize;

    for hull in &mesh.hulls {
        if hull.len() < 4 {
            continue;
        }
        let start = data.len();
        for p in hull {
            data.extend_from_slice(p);
        }
        shapes.push_str(&hull_shape(start, data.len() - start));
        count += 1;
    }

    if count == 0 && !mesh.triangles.is_empty() {
        let (center, half) = aabb(&mesh.triangles);
        let start = data.len();
        data.extend_from_slice(&half);
        shapes.push_str(&box_shape(center, start, 3));
        count += 1;
    }

    if count == 0 {
        return None;
    }

    let is_sensor = entity.collision.as_ref().is_some_and(|c| c.sensor)
        || entity.classname.starts_with("trigger");
    let kind = match text_property(entity, "defold_body_type") {
        Some("static") => "COLLISION_OBJECT_TYPE_STATIC",
        Some("kinematic") => "COLLISION_OBJECT_TYPE_KINEMATIC",
        Some("dynamic") => "COLLISION_OBJECT_TYPE_DYNAMIC",
        Some("trigger") => "COLLISION_OBJECT_TYPE_TRIGGER",
        _ => {
            if is_sensor {
                "COLLISION_OBJECT_TYPE_TRIGGER"
            } else if is_mover(&entity.classname)
                || entity.properties.contains_key("_heron_animation")
            {
                "COLLISION_OBJECT_TYPE_KINEMATIC"
            } else {
                "COLLISION_OBJECT_TYPE_STATIC"
            }
        }
    };

    let mut out = String::new();
    out.push_str(&format!("type: {kind}\n"));
    out.push_str(&format!(
        "mass: {}\nfriction: {}\nrestitution: {}\n",
        f(number_property(entity, "defold_mass", 0.)),
        f(number_property(entity, "defold_friction", 0.1)),
        f(number_property(entity, "defold_restitution", 0.5))
    ));
    out.push_str(&format!(
        "group: \"{}\"\n",
        escape(text_property(entity, "defold_group").unwrap_or("default"))
    ));
    // The generated trigger protocol uses the player group. Both sides of a
    // Defold collision must permit it, including the solid floor and movers.
    for mask in text_property(entity, "defold_mask")
        .unwrap_or("default,player")
        .split([',', ' '])
        .filter(|v| !v.is_empty())
    {
        out.push_str(&format!("mask: \"{}\"\n", escape(mask)));
    }
    out.push_str(&format!(
        "linear_damping: {}\nangular_damping: {}\n",
        f(number_property(entity, "defold_linear_damping", 0.)),
        f(number_property(entity, "defold_angular_damping", 0.))
    ));
    let locked = match entity.properties.get("defold_locked_rotation") {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(v)) => v == "true" || v == "1",
        _ => true,
    };
    out.push_str(&format!("locked_rotation: {locked}\n"));
    out.push_str("embedded_collision_shape {\n");
    out.push_str(&shapes);
    for v in &data {
        out.push_str(&format!("  data: {}\n", f(*v as f64)));
    }
    out.push_str("}\n");
    Some(out)
}

fn hull_shape(index: usize, count: usize) -> String {
    format!(
        "  shapes {{\n    shape_type: TYPE_HULL\n    position {{\n    }}\n    \
         rotation {{\n    }}\n    index: {index}\n    count: {count}\n  }}\n"
    )
}

fn box_shape(center: [f32; 3], index: usize, count: usize) -> String {
    format!(
        "  shapes {{\n    shape_type: TYPE_BOX\n    position {{\n      x: {}\n      y: {}\n      \
         z: {}\n    }}\n    rotation {{\n    }}\n    index: {index}\n    count: {count}\n  }}\n",
        f(center[0] as f64),
        f(center[1] as f64),
        f(center[2] as f64),
    )
}

/// Centre and half-extents of the box around a point list (flat x,y,z).
fn aabb(points: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in points {
        for a in 0..3 {
            lo[a] = lo[a].min(p[a]);
            hi[a] = hi[a].max(p[a]);
        }
    }
    let center = [
        (lo[0] + hi[0]) * 0.5,
        (lo[1] + hi[1]) * 0.5,
        (lo[2] + hi[2]) * 0.5,
    ];
    let half = [
        ((hi[0] - lo[0]) * 0.5).max(0.01),
        ((hi[1] - lo[1]) * 0.5).max(0.01),
        ((hi[2] - lo[2]) * 0.5).max(0.01),
    ];
    (center, half)
}

fn is_mover(classname: &str) -> bool {
    matches!(
        classname,
        "func_door"
            | "func_door_rotating"
            | "func_button"
            | "func_train"
            | "func_plat"
            | "func_rotating"
            | "func_bobbing"
            | "func_pendulum"
            | "func_mover"
            | "fps_lift"
            | "fps_door"
            | "fps_key_door"
    )
}

// ---------------------------------------------------------------------------
// Script (a point entity's keys, so gameplay code can go.get them)

/// Map keys already spent on the game object's transform, identity or
/// links, so they do not go out again as gameplay properties.
fn is_reserved_key(key: &str) -> bool {
    matches!(
        key,
        "classname" | "origin" | "angles" | "angle" | "mangle" | "rotation" | "scale" | "target"
    )
}

fn script_file(
    entity: &crate::level::Entity,
    ids: &std::collections::HashMap<String, String>,
    water_module: &str,
) -> Option<String> {
    let props: Vec<(&String, &Value)> = entity
        .properties
        .iter()
        .filter(|(k, v)| {
            !engine_hint(k)
                && !k.starts_with("animation_")
                && !k.starts_with("_heron_")
                && !is_reserved_key(k)
                && !matches!(v, Value::String(s) if s.is_empty())
        })
        .collect();
    if props.is_empty() && entity.targets.is_empty() && entity.classname == "worldspawn" {
        return None;
    }

    let mut out = String::new();
    out.push_str(
        "-- Generated by `heron_brush export --engine defold`. Re-exporting overwrites this file.\n",
    );
    out.push_str(&format!(
        "go.property(\"classname\", hash(\"{}\"))\n",
        lua_str(&entity.classname)
    ));
    if let Some(target) = entity.targets.first() {
        out.push_str(&format!(
            "go.property(\"target\", hash(\"{}\"))\n",
            lua_str(target)
        ));
    }
    for (key, value) in props {
        let id = ident(key);
        match value {
            Value::Bool(b) => out.push_str(&format!("go.property(\"{id}\", {b})\n")),
            Value::Int(i) => out.push_str(&format!("go.property(\"{id}\", {i})\n")),
            Value::Float(x) => out.push_str(&format!("go.property(\"{id}\", {})\n", f(*x))),
            Value::String(s) => out.push_str(&format!(
                "go.property(\"{id}\", hash(\"{}\"))\n",
                lua_str(s)
            )),
            Value::Vec2([x, y]) => out.push_str(&format!(
                "go.property(\"{id}\", vmath.vector3({}, {}, 0))\n",
                f(*x),
                f(*y)
            )),
            Value::Vec3([x, y, z]) => out.push_str(&format!(
                "go.property(\"{id}\", vmath.vector3({}, {}, {}))\n",
                f(*x),
                f(*y),
                f(*z)
            )),
            Value::Color([r, g, b, a]) => out.push_str(&format!(
                "go.property(\"{id}\", vmath.vector4({}, {}, {}, {}))\n",
                f(*r),
                f(*g),
                f(*b),
                f(*a)
            )),
            Value::Array(_) => {}
        }
    }
    if let Some(Value::String(data)) = entity.properties.get("_heron_animation") {
        out.push_str(&format!(
            "\nlocal hb_data = json.decode(\"{}\")\n",
            lua_str(data)
        ));
        out.push_str(include_str!("defold_animation.script"));
    }
    if let Some(Value::String(data)) = entity.properties.get("_heron_trigger") {
        let mut data: serde_json::Value = serde_json::from_str(data).expect("validated trigger");
        if let Some(targets) = data["targets"].as_array_mut() {
            for name in targets {
                if let Some(s) = name.as_str() {
                    *name = serde_json::json!(ids.get(s).cloned().unwrap_or_else(|| ident(s)));
                }
            }
        }
        out.push_str(&format!(
            "\nlocal hb_trigger=json.decode(\"{}\")\n",
            lua_str(&data.to_string())
        ));
        out.push_str(include_str!("defold_animation_trigger.script"));
    }
    if let Some(Value::String(data)) = entity.properties.get("_heron_water") {
        out.push_str(&format!(
            "\nlocal hb_water=require \"{}\"\nlocal hb_water_data=json.decode(\"{}\")\n",
            lua_str(water_module),
            lua_str(data)
        ));
        if entity.properties.contains_key("defold_material") {
            out.push_str("hb_water_data.surface_material=false\n");
        }
        out.push_str("local water_init,water_update,water_final=init,update,final\nfunction init(self) if water_init then water_init(self) end;self.hb_water_time=0;hb_water.add(go.get_id(),hb_water_data) end\nfunction final(self) hb_water.remove(go.get_id());if water_final then water_final(self) end end\nfunction update(self,dt) if water_update then water_update(self,dt) end;self.hb_water_time=self.hb_water_time+dt;if hb_water_data.surface_material then go.set(\"#mesh\",\"water_tint\",vmath.vector4(hb_water_data.color[1],hb_water_data.color[2],hb_water_data.color[3],hb_water_data.opacity));go.set(\"#mesh\",\"water_params\",vmath.vector4(self.hb_water_time,hb_water_data.wave_height,hb_water_data.wave_speed,0)) end end\n");
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Helpers

/// heron_brush's `rotation_yxz` (radians about X, Y, Z, applied Y then X then Z) to a
/// quaternion `[x, y, z, w]`.
fn quat_yxz(e: [f64; 3]) -> [f64; 4] {
    let (sx, cx) = (e[0] * 0.5).sin_cos();
    let (sy, cy) = (e[1] * 0.5).sin_cos();
    let (sz, cz) = (e[2] * 0.5).sin_cos();
    let qx = [sx, 0.0, 0.0, cx];
    let qy = [0.0, sy, 0.0, cy];
    let qz = [0.0, 0.0, sz, cz];
    // Match Godot EULER_ORDER_YXZ and glam EulerRot::YXZ.
    mul(mul(qy, qx), qz)
}

fn mul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

/// A Defold identifier: keep `[A-Za-z0-9_-]`, fold the rest to `_`.
fn ident(raw: &str) -> String {
    let mut s: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() {
        s.push('_');
    }
    s
}

fn unique(used: &mut std::collections::HashMap<String, u32>, id: String) -> String {
    let n = used.entry(id.clone()).or_insert(0);
    *n += 1;
    if *n == 1 {
        id
    } else {
        format!("{id}_{}", *n - 1)
    }
}

/// `prefix` + `rel`, collapsing a doubled slash at the seam.
fn join_resource(prefix: &str, rel: &str) -> String {
    let rel = rel.trim_start_matches('/');
    if prefix.ends_with('/') {
        format!("{prefix}{rel}")
    } else {
        format!("{prefix}/{rel}")
    }
}

/// Escape a string for a Defold protobuf `data:` field.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(c),
        }
    }
    out
}

fn lua_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\n', '\r'], " ")
}

/// A protobuf float literal: always with a decimal point, no exponent for the
/// everyday range.
fn f(v: f64) -> String {
    if !v.is_finite() || v.abs() < 1e-7 {
        return "0.0".to_string();
    }
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}.0", v as i64)
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{Collision, CollisionShape, Entity, Surface, Texture, Transform};

    fn opts() -> DefoldOptions {
        DefoldOptions {
            asset_dir: "/main/arena".into(),
            ..Default::default()
        }
    }

    #[test]
    fn project_materials_receive_explicit_pbr_samplers_without_game_properties() {
        let mut level = level();
        let entity = &mut level.entities[0];
        for (slot, sampler) in [
            ("normal", "tex_normal"),
            ("mr", "tex_mr"),
            ("ao", "tex_ao"),
            ("emissive", "tex_emissive"),
        ] {
            entity.properties.insert(
                format!("defold_sampler_{slot}"),
                Value::String(sampler.into()),
            );
        }
        entity.properties.insert(
            "defold_material".into(),
            Value::String("/main/pbr.material".into()),
        );
        let mut options = opts();
        options.material_maps.insert(
            "wall".into(),
            ["normal", "mr", "ao", "emissive"]
                .into_iter()
                .map(|slot| (slot.into(), format!("textures/wall_{slot}.png")))
                .collect(),
        );
        validate(&level).unwrap();
        validate_material_bindings(&level, &options).unwrap();
        let mut missing = options.clone();
        missing.material_maps.clear();
        assert!(validate_material_bindings(&level, &missing).is_err());
        let files = write(&level, &[], &options);
        let model = &files
            .iter()
            .find(|f| f.path == "arena/worldspawn.model")
            .unwrap()
            .contents;
        for slot in ["normal", "mr", "ao", "emissive"] {
            assert!(
                model.contains(&format!("sampler: \"tex_{slot}\"")),
                "{model}"
            );
            assert!(
                model.contains(&format!("texture: \"/textures/wall_{slot}.png\"")),
                "{model}"
            );
        }
        assert!(
            files
                .iter()
                .filter(|f| f.path.ends_with(".script"))
                .all(|f| !f.contents.contains("go.property(\"defold_sampler_"))
        );
        level.entities[0].properties.insert(
            "defold_sampler_normal".into(),
            Value::String("invalid sampler".into()),
        );
        assert!(validate(&level).is_err());
        level.entities[0].properties.insert(
            "defold_sampler_normal".into(),
            Value::String("tex_mr".into()),
        );
        assert!(validate(&level).is_err());
    }

    #[test]
    fn a_sky_is_a_cubemap_on_a_model_the_render_script_draws_first() {
        let sky = Sky {
            sky: [0.1, 0.4, 0.85],
            horizon: [0.7, 0.8, 0.9],
            ground: [0.2, 0.18, 0.15],
            sun_color: [1., 0.9, 0.8],
            sun_intensity: 3.,
            ambient_intensity: 0.5,
            sun_elevation: 45.,
            sun_azimuth: 0.,
            exposure: 1.,
            fog: None,
        };
        let files = write(
            &level(),
            &[],
            &DefoldOptions {
                sky: Some(sky.clone()),
                ..opts()
            },
        );
        let file = |path: &str| {
            files
                .iter()
                .find(|f| f.path == path)
                .unwrap_or_else(|| panic!("{path}"))
                .contents
                .clone()
        };
        assert!(file("arena.collection").contains("id: \"heron_sky\""));
        assert!(file("arena/sky.cubemap").contains("front: \"/main/arena/sky_front.png\""));
        assert!(file("arena/sky.model").contains("texture: \"/main/arena/sky.cubemap\""));
        assert!(file("arena/sky.gltf").contains("data:application/octet-stream;base64,"));
        assert!(file("arena.render").contains("/main/arena.render_script"));
        let script = file("arena.render_script");
        assert!(
            script.find("render.draw(self.sky)").unwrap()
                < script.find("render.draw(self.model)").unwrap()
        );
        assert!(file("arena_environment.lua").contains("sun_intensity = 3"));
        let images = sky_binaries("arena", &sky);
        assert_eq!(images.len(), 6);
        assert!(
            images
                .iter()
                .all(|(path, png)| path.starts_with("arena/sky_") && png.starts_with(b"\x89PNG"))
        );
        // Without a sky or water nothing replaces the project's renderer.
        assert!(
            !write(&level(), &[], &opts())
                .iter()
                .any(|f| f.path.ends_with(".render"))
        );
    }

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
                        kind: CollisionShape::Convex,
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

    fn hull_cube() -> CollisionMesh {
        let c = [
            [-1.0, -1.0, -1.0f32],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];
        CollisionMesh {
            hulls: vec![c.to_vec()],
            ..Default::default()
        }
    }

    #[test]
    fn project_hints_support_custom_physics_and_components_without_game_classnames() {
        let mut level = level();
        let entity = &mut level.entities[0];
        entity.classname = "relic".into();
        for (key, value) in [
            ("defold_body_type", "dynamic"),
            ("defold_group", "artifact"),
            ("defold_mask", "geometry,artifact"),
            ("defold_component_logic", "/game/relic.script"),
            ("defold_material", "/game/stone.material"),
            ("defold_sampler", "albedo"),
        ] {
            entity
                .properties
                .insert(key.into(), Value::String(value.into()));
        }
        entity
            .properties
            .insert("defold_mass".into(), Value::Float(2.));
        assert!(validate(&level).is_ok());
        let files = write(&level, &[hull_cube(), CollisionMesh::default()], &opts());
        let collision = &files
            .iter()
            .find(|f| f.path.ends_with("worldspawn.collisionobject"))
            .unwrap()
            .contents;
        assert!(collision.contains("COLLISION_OBJECT_TYPE_DYNAMIC"));
        assert!(collision.contains("mass: 2"));
        assert!(collision.contains("group: \"artifact\""));
        assert!(collision.contains("mask: \"geometry\""));
        assert!(files[0].contents.contains("/game/relic.script"));
        assert!(
            files
                .iter()
                .find(|f| f.path.ends_with("worldspawn.model"))
                .unwrap()
                .contents
                .contains("/game/stone.material")
        );
        let script = &files
            .iter()
            .find(|f| f.path.ends_with("worldspawn.script"))
            .unwrap()
            .contents;
        assert!(!script.contains("defold_component_logic"));
        level.entities[0]
            .properties
            .insert("defold_body_type".into(), Value::String("none".into()));
        level.entities[0].properties.shift_remove("defold_mass");
        assert!(
            !write(&level, &[hull_cube(), CollisionMesh::default()], &opts())
                .iter()
                .any(|f| f.path.ends_with("worldspawn.collisionobject"))
        );
    }

    #[test]
    fn invalid_project_hints_are_reported_instead_of_silently_ignored() {
        for (key, value) in [
            ("defold_body_type", Value::String("dynamci".into())),
            ("defold_mass", Value::Float(f64::NAN)),
            ("defold_restitution", Value::Float(2.)),
            (
                "defold_component_mesh",
                Value::String("/custom.model".into()),
            ),
        ] {
            let mut level = level();
            level.entities[0].properties.insert(key.into(), value);
            assert!(validate(&level).is_err(), "{key}");
        }
        let mut level = level();
        level.entities[0]
            .properties
            .insert("defold_body_type".into(), Value::String("dynamic".into()));
        assert!(validate(&level).is_err());
    }

    #[test]
    fn the_collection_comes_first_and_names_the_level() {
        let cm = [hull_cube(), CollisionMesh::default()];
        let files = write(&level(), &cm, &opts());
        assert_eq!(files[0].path, "arena.collection");
        assert!(files[0].contents.starts_with("name: \"arena\"\n"));
        assert!(
            files[0]
                .contents
                .contains("embedded_instances {\n  id: \"worldspawn\"\n")
        );
    }

    #[test]
    fn a_solid_entity_gets_a_model_and_a_collisionobject() {
        let cm = [hull_cube(), CollisionMesh::default()];
        let files = write(&level(), &cm, &opts());
        let model = files
            .iter()
            .find(|f| f.path == "arena/worldspawn.model")
            .unwrap();
        assert!(
            model
                .contents
                .contains("mesh: \"/main/arena/worldspawn.glb\"\n")
        );
        assert!(model.contents.contains("name: \"wall\"\n"));
        assert!(model.contents.contains("texture: \"/textures/wall.png\"\n"));

        let co = files
            .iter()
            .find(|f| f.path == "arena/worldspawn.collisionobject")
            .unwrap();
        assert!(co.contents.contains("type: COLLISION_OBJECT_TYPE_STATIC\n"));
        assert!(co.contents.contains("shape_type: TYPE_HULL\n"));
        assert!(co.contents.contains("count: 24\n"));
        assert_eq!(co.contents.matches("  data: ").count(), 24);
    }

    #[test]
    fn a_point_entity_gets_a_script_with_its_class() {
        let mut l = level();
        l.entities[1]
            .properties
            .insert("health".into(), Value::Float(100.0));
        let files = write(&l, &[], &opts());
        let script = files
            .iter()
            .find(|f| f.path == "arena/enemy0.script")
            .unwrap();
        assert!(
            script
                .contents
                .contains("go.property(\"classname\", hash(\"fps_enemy\"))\n")
        );
        assert!(script.contents.contains("go.property(\"health\", 100.0)\n"));

        let col = &files[0].contents;
        assert!(col.contains("id: \"enemy0\""));
        assert!(col.contains("component: \\\"/main/arena/enemy0.script\\\""));
    }

    #[test]
    fn a_trigger_is_a_trigger_object() {
        let mut l = level();
        l.entities[0].classname = "trigger_multiple".into();
        l.entities[0].collision.as_mut().unwrap().sensor = true;
        let cm = [hull_cube(), CollisionMesh::default()];
        let files = write(&l, &cm, &opts());
        let co = files
            .iter()
            .find(|f| f.path.ends_with(".collisionobject"))
            .unwrap();
        assert!(
            co.contents
                .contains("type: COLLISION_OBJECT_TYPE_TRIGGER\n")
        );
    }
    #[test]
    fn custom_material_and_sampler_reach_model_resources() {
        let mut options = opts();
        options.model_material = "/main/custom.material".into();
        options.model_sampler = "albedo".into();
        let files = write(&level(), &[], &options);
        let model = &files
            .iter()
            .find(|f| f.path.ends_with(".model"))
            .unwrap()
            .contents;
        assert!(model.contains("/main/custom.material"));
        assert!(model.contains("albedo"));
    }

    #[test]
    fn mixed_rotation_and_scale_match_engine_neutral_transform() {
        let angles = [0.3, -0.7, 0.4];
        let expected =
            glam::DQuat::from_euler(glam::EulerRot::YXZ, angles[1], angles[0], angles[2])
                .to_array();
        for (actual, expected) in quat_yxz(angles).iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-12);
        }
        let mut entity = crate::level::Entity::default();
        entity.transform.scale = [2.0, 0.5, 3.0];
        let collection = embedded_instance("scaled", &entity, &[]);
        assert!(
            collection.contains("scale3 {\n    x: 2.0\n    y: 0.5\n    z: 3.0"),
            "{collection}"
        );
    }
}
