//! Native Flax reflection captures; diffuse volumes select Flax's dynamic GI.
use super::{Contents, Error};
use crate::document::Document;
use serde_json::{json, Value};
use std::path::Path;
pub(super) fn probes(
    document: &Document,
    units: f64,
    directory: &Path,
    name: &str,
    files: &mut Vec<(std::path::PathBuf, Contents)>,
) -> Result<Value, Error> {
    let config = crate::light_probes::settings(document);
    if config.probes.is_empty() {
        return Ok(json!([]));
    }
    let mut capture = crate::render_settings::for_document(document);
    if let Some(lighting) = &config.lighting {
        lighting.apply(&mut capture);
        capture.sun_elevation = config.sun_elevation;
        capture.sun_azimuth = config.sun_azimuth;
        capture.shadows = config.shadows;
    }
    let baked =
        crate::light_probes::bake_blocking(document, &config, &capture).map_err(Error::Map)?;
    let mut probes = Vec::new();
    for probe in baked.probes() {
        let (bytes, range) = scene_export::writer::sky::cube_dds_mips_normalized(
            probe.size,
            probe.mips.len() as u32,
            true,
            |direction, mip| {
                sample(
                    probe,
                    scene_export::writer::sky::from_export(direction),
                    mip as usize,
                )
            },
        );
        let path = format!("probe_{}.dds", super::paint_export::image_name(&bytes));
        files.push((directory.join(name).join(&path), Contents::Bytes(bytes)));
        let v = &probe.volume;
        let factor = 100.0 / units as f32;
        probes.push(json!({"texture":path, "position":[v.position[1]*factor,v.position[2]*factor,-v.position[0]*factor],
            "half_size":[v.half_size[1]*factor,v.half_size[2]*factor,v.half_size[0]*factor],
            "blend":v.blend,"brightness":range*v.intensity,"reflections":v.reflections,"diffuse":v.diffuse}));
    }
    Ok(json!(probes))
}
fn sample(probe: &render::probes::BakedProbe, d: [f32; 3], mip: usize) -> [f32; 3] {
    let [x, y, z] = d;
    let [ax, ay, az] = d.map(f32::abs);
    let (face, u, v) = if ax >= ay && ax >= az {
        if x >= 0. {
            (0, -z / ax, -y / ax)
        } else {
            (1, z / ax, -y / ax)
        }
    } else if ay >= az {
        if y >= 0. {
            (2, x / ay, z / ay)
        } else {
            (3, x / ay, -z / ay)
        }
    } else if z >= 0. {
        (4, x / az, -y / az)
    } else {
        (5, -x / az, -y / az)
    };
    let size = (probe.size >> mip).max(1) as usize;
    let coordinate = |v: f32| (((v * 0.5 + 0.5) * size as f32) as usize).min(size - 1);
    let p = probe.mips[mip][face * size * size + coordinate(v) * size + coordinate(u)];
    [p[0], p[1], p[2]]
}

/// Surface unsupported layers in the export report instead of silently losing them.
pub(super) fn material_warnings(files: &[(std::path::PathBuf, Contents)]) -> Vec<String> {
    let mut coat = false;
    let mut anisotropy = false;
    for (path, contents) in files {
        if !path.extension().is_some_and(|e| e == "glb") {
            continue;
        }
        let Contents::Bytes(bytes) = contents else {
            continue;
        };
        if bytes.len() < 20 {
            continue;
        }
        let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let Some(json) = bytes.get(20..20 + length) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(json) else {
            continue;
        };
        if let Some(materials) = value["materials"].as_array() {
            for material in materials {
                coat |= material["extensions"]
                    .get("KHR_materials_clearcoat")
                    .is_some();
                anisotropy |= material["extensions"]
                    .get("KHR_materials_anisotropy")
                    .is_some();
            }
        }
    }
    let mut warnings = Vec::new();
    if coat {
        warnings.push("Flax Lit materials do not support clear coat; the GLB retains the layer for a custom material.".into());
    }
    if anisotropy {
        warnings.push("Flax Lit materials do not support anisotropy; the GLB retains the layer for a custom material.".into());
    }
    warnings
}
