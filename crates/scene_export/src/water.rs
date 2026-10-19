//! Portable water volumes. Geometry is entity-local; speeds and heights are metres.
use crate::{
    level::{Entity, Value},
    map, math,
};
use serde::Serialize;

pub fn enabled(entity: &map::Entity) -> bool {
    match entity.get("water_enabled") {
        Some(value) => matches!(value.trim(), "1" | "true" | "yes"),
        None => matches!(
            entity.classname(),
            "water_volume" | "fps_water" | "func_water"
        ),
    }
}
pub fn baseline(source: &map::Entity) -> bool {
    !matches!(
        source.get("water_surface_material").map(str::trim),
        Some("0" | "false" | "no")
    )
}
#[derive(Serialize)]
pub struct Plane {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub d: f64,
}
#[derive(Serialize)]
pub struct Volume {
    pub planes: Vec<Plane>,
    pub surface: f64,
}
#[derive(Serialize)]
pub struct SurfaceVertex {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub nx: f64,
    pub ny: f64,
    pub nz: f64,
    pub u: f64,
    pub v: f64,
}
#[derive(Serialize)]
pub struct Water {
    pub version: u32,
    pub volumes: Vec<Volume>,
    pub surface_vertices: Vec<SurfaceVertex>,
    pub surface_indices: Vec<u32>,
    pub swim_speed: f64,
    pub rise_speed: f64,
    pub sink_speed: f64,
    pub drag: f64,
    pub priority: i32,
    pub color: [f64; 3],
    pub opacity: f64,
    pub fog_density: f64,
    pub wave_height: f64,
    pub wave_speed: f64,
    pub surface_material: bool,
}
pub fn build(source: &map::Entity, entity: &Entity, scale: f64) -> Option<Water> {
    if !enabled(source) || !scale.is_finite() || scale <= 0. {
        return None;
    }
    let t = &entity.transform;
    let rotation = glam::DQuat::from_euler(
        glam::EulerRot::YXZ,
        t.rotation_yxz[1],
        t.rotation_yxz[0],
        t.rotation_yxz[2],
    );
    let matrix = glam::DMat4::from_scale_rotation_translation(
        glam::DVec3::from(t.scale),
        rotation,
        glam::DVec3::from(t.translation),
    );
    if matrix.determinant().abs() < 1e-12 {
        return None;
    }
    let inverse = matrix.inverse();
    let mut volumes = Vec::new();
    let mut surface_vertices = Vec::new();
    let mut surface_indices = Vec::new();
    for brush in &source.brushes {
        if brush.faces.len() < 4 {
            continue;
        }
        let mut points = Vec::new();
        let mut planes = Vec::new();
        for face in &brush.faces {
            let n = math::to_godot(face.plane.normal);
            let d = face.plane.d * scale;
            let local = matrix.transpose() * glam::DVec4::new(n.x, n.y, n.z, -d);
            let length = local.truncate().length();
            if length < 1e-12 {
                return None;
            }
            planes.push(Plane {
                x: local.x / length,
                y: local.y / length,
                z: local.z / length,
                d: -local.w / length,
            });
            let mut polygon = math::base_winding(&face.plane, math::DEFAULT_HYPERPLANE_SIZE);
            for clip in &brush.faces {
                if !std::ptr::eq(face, clip) {
                    polygon = math::clip_polygon(&polygon, &clip.plane, 1e-7);
                }
            }
            let local_points: Vec<_> = polygon
                .into_iter()
                .map(|p| inverse.transform_point3(math::to_godot(p) * scale))
                .collect();
            let base = surface_vertices.len() as u32;
            for p in &local_points {
                surface_vertices.push(SurfaceVertex {
                    x: p.x,
                    y: p.y,
                    z: p.z,
                    nx: local.x / length,
                    ny: local.y / length,
                    nz: local.z / length,
                    u: if local.y.abs() > length * 0.5 {
                        p.x
                    } else {
                        p.x + p.z
                    },
                    v: if local.y.abs() > length * 0.5 {
                        p.z
                    } else {
                        p.y
                    },
                });
            }
            for i in (1..local_points.len().saturating_sub(1)).filter(|_| n.y > 0.5) {
                surface_indices.extend([base, base + i as u32, base + i as u32 + 1]);
            }
            points.extend(local_points);
        }
        let surface = points.iter().map(|p| p.y).reduce(f64::max)?;
        if !surface.is_finite() {
            return None;
        }
        volumes.push(Volume { planes, surface });
    }
    if volumes.is_empty() {
        return None;
    }
    let number = |key: &str, default: f64, min: f64, max: f64| {
        source
            .get(key)
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|n| n.is_finite())
            .unwrap_or(default)
            .clamp(min, max)
    };
    let color = source
        .get("water_color")
        .map(|s| {
            s.split_whitespace()
                .filter_map(|v| v.parse::<f64>().ok())
                .collect::<Vec<_>>()
        })
        .filter(|v| v.len() == 3 && v.iter().all(|n| n.is_finite()))
        .map(|v| [v[0].clamp(0., 1.), v[1].clamp(0., 1.), v[2].clamp(0., 1.)])
        .unwrap_or([0.05, 0.38, 0.48]);
    Some(Water {
        version: 1,
        volumes,
        surface_vertices,
        surface_indices,
        swim_speed: number("swim_speed", 3., 0., 100.),
        rise_speed: number("rise_speed", 3.5, 0., 100.),
        sink_speed: number("sink_speed", 0.8, 0., 100.),
        drag: number("water_drag", 8., 0., 100.),
        priority: number("water_priority", 0., -10000., 10000.) as i32,
        color,
        opacity: number("water_opacity", 0.55, 0., 1.),
        fog_density: number("water_fog_density", 0.18, 0., 100.),
        wave_height: number("wave_height", 0.035, 0., 1.),
        wave_speed: number("wave_speed", 1.5, 0., 100.),
        surface_material: baseline(source),
    })
}
/// The 3x4 matrix, by rows, that takes a world point into the frame of an entity at
/// `transform`: where a volume's planes are.
pub fn world_to_local(transform: &crate::level::Transform) -> [f64; 12] {
    let t = transform;
    let rotation = glam::DQuat::from_euler(glam::EulerRot::YXZ, t.rotation_yxz[1], t.rotation_yxz[0], t.rotation_yxz[2]);
    let inverse = glam::DMat4::from_scale_rotation_translation(glam::DVec3::from(t.scale), rotation, glam::DVec3::from(t.translation)).inverse();
    std::array::from_fn(|i| inverse.col(i % 4)[i / 4])
}
pub fn attach(source: &map::Entity, entity: &mut Entity, scale: f64, warnings: &mut Vec<String>) {
    entity.properties.shift_remove("_heron_water");
    if !enabled(source) {
        return;
    }
    if let Some(water) = build(source, entity, scale) {
        if let Some(collision) = &mut entity.collision {
            collision.sensor = true;
        }
        entity.properties.insert(
            "_heron_water".into(),
            Value::String(serde_json::to_string(&water).unwrap()),
        );
    } else {
        warnings.push(format!(
            "Water {}: no valid closed brush volume; water runtime omitted",
            entity.name
        ));
    }
}

/// Installs the optional baseline transparent water material in generated GLBs.
/// Game-owned materials can disable this through water_surface_material=0.
pub fn glb_material(bytes: &mut Vec<u8>, water: &Water) {
    if !water.surface_material || bytes.len() < 20 || bytes.get(..4) != Some(b"glTF") {
        return;
    }
    let size = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let mut json: serde_json::Value =
        serde_json::from_slice(&bytes[20..20 + size]).expect("generated GLB JSON");
    if let Some(materials) = json["materials"].as_array_mut() {
        for material in materials {
            material["alphaMode"] = serde_json::json!("BLEND");
            material["doubleSided"] = serde_json::json!(true);
            material["pbrMetallicRoughness"]["baseColorFactor"] = serde_json::json!([
                water.color[0],
                water.color[1],
                water.color[2],
                water.opacity
            ]);
            material["pbrMetallicRoughness"]["roughnessFactor"] = serde_json::json!(0.14);
            material["pbrMetallicRoughness"]["metallicFactor"] = serde_json::json!(0.05);
        }
    }
    let mut encoded = serde_json::to_vec(&json).unwrap();
    while encoded.len() % 4 != 0 {
        encoded.push(b' ');
    }
    let mut result = bytes[..12].to_vec();
    result.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
    result.extend_from_slice(b"JSON");
    result.extend(encoded);
    result.extend_from_slice(&bytes[20 + size..]);
    let size = result.len() as u32;
    result[8..12].copy_from_slice(&size.to_le_bytes());
    *bytes = result;
}
