//! Adds imported model geometry to the common export build, before engine writers.
use crate::{document::Document, export::Error};
use scene_export::{
    geometry::{Surface, Vertex},
    map::MapFile,
    math::{Vec2, Vec3},
    settings::Settings,
};
pub fn prepare(
    document: &Document,
    map: &mut MapFile,
    settings: &mut Settings,
) -> Result<(), Error> {
    let mut source_surfaces: std::collections::HashMap<
        std::path::PathBuf,
        (std::sync::Arc<[Surface]>, Vec<String>),
    > = std::collections::HashMap::new();
    for entity in &mut map.entities {
        if entity.get("_heron_model").is_none() {
            continue;
        }
        let source = map_model::node::EntityNode {
            properties: entity
                .properties
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            children: vec![],
        };
        let model = document.imported_model(&source).ok_or_else(|| {
            Error::Map(format!(
                "Imported model '{}' is unavailable or has invalid transforms",
                entity.get("_heron_model").unwrap()
            ))
        })?;
        let origin = source
            .origin()
            .unwrap_or_else(geometry_math::vec::Vec3d::zero);
        let mut texture_names = None;
        if let Some(raw) = document.imported_model_source(&source) {
            let info = map_model::entity_rotation::RotationInfo {
                kind: map_model::entity_rotation::RotationType::Euler,
                property_key: "angles".into(),
                usage: map_model::entity_rotation::RotationUsage::Allowed,
            };
            let rotation = map_model::entity_rotation::rotation(&source.properties, &info);
            let axes = [
                geometry_math::vec::Vec3d::new(1., 0., 0.),
                geometry_math::vec::Vec3d::new(0., 1., 0.),
                geometry_math::vec::Vec3d::new(0., 0., 1.),
            ];
            let rotation = axes.map(|a| {
                let p = rotation * a;
                [p.x(), p.y(), p.z()]
            });
            let values = source
                .property("modelscale")
                .unwrap_or("1")
                .split_whitespace()
                .filter_map(|s| s.parse::<f64>().ok())
                .collect::<Vec<_>>();
            let scale = match values.as_slice() {
                [s] => [*s; 3],
                [x, y, z] => [*x, *y, *z],
                _ => [1.; 3],
            };
            let path = document.imported_model_path(&source).unwrap();
            let (shared, names) = if let Some(cached) = source_surfaces.get(&path) {
                cached.clone()
            } else {
                let (surfaces, names) =
                    model_surfaces(&raw, geometry_math::vec::Vec3d::zero(), settings, None)?;
                let shared: std::sync::Arc<[Surface]> = surfaces.into();
                source_surfaces.insert(path, (shared.clone(), names.clone()));
                (shared, names)
            };
            texture_names = Some(names);
            entity.external_asset = Some(scene_export::instancing::ModelAsset {
                surfaces: shared,
                rotation,
                scale,
            });
        }
        let (surfaces, _) = model_surfaces(&model, origin, settings, texture_names.as_deref())?;
        entity
            .external_surfaces
            .extend(surfaces.into_iter().filter(|s| !s.indices.is_empty()));
    }
    Ok(())
}

fn model_surfaces(
    model: &map_model::entity_model::EntityModel,
    origin: geometry_math::vec::Vec3d,
    settings: &mut Settings,
    known_names: Option<&[String]>,
) -> Result<(Vec<Surface>, Vec<String>), Error> {
    let mut surfaces = Vec::new();
    for (material_index, skin) in model.skins().enumerate() {
        let texture = if let Some(names) = known_names {
            names[material_index].clone()
        } else {
            let mut hasher = crate::export_hash::ExportHasher::default();
            use std::hash::{Hash, Hasher};
            skin.pixels.hash(&mut hasher);
            skin.width.hash(&mut hasher);
            skin.height.hash(&mut hasher);
            if let Some(pbr) = model.pbr_at(material_index) {
                pbr.hash(&mut hasher);
            }
            let texture = format!("heron_import_{:016x}", hasher.finish());
            texture
        };
        if let scene_export::indexmap::map::Entry::Vacant(entry) =
            settings.generated_materials.entry(texture.clone())
        {
            entry.insert(scene_export::materials::GeneratedMaterial {
                pbr: model
                    .pbr_at(material_index)
                    .map(generated_pbr)
                    .transpose()?,
                path: format!("textures/{texture}.png"),
                width: skin.width,
                height: skin.height,
                png: crate::assets::encode_png(skin.width, skin.height, &skin.pixels)
                    .map_err(Error::Map)?,
                alpha: skin.pixels.chunks_exact(4).any(|p| p[3] != 255),
                surface: None,
                repeat: true,
            });
        }
        surfaces.push(Surface {
            texture,
            vertices: vec![],
            indices: vec![],
        });
    }

    for triangle in &model.triangles {
        let skin = model.skin_at(triangle.material);
        let surface = &mut surfaces[triangle.material];
        let normal = geometry_math::vec::cross(
            triangle.positions[1] - triangle.positions[0],
            triangle.positions[2] - triangle.positions[0],
        );
        let length = geometry_math::vec::length(normal);
        if length < 1e-9 {
            continue;
        }
        let normal = normal / length;
        let base = surface.vertices.len() as u32;
        for i in 0..3 {
            let p = triangle.positions[i] + origin;
            let uv = triangle.uv[i];
            let normal = triangle.normals.map_or(normal, |n| n[i]);
            surface.vertices.push(Vertex {
                position: Vec3::new(p.x(), p.y(), p.z()),
                normal: Vec3::new(normal.x(), normal.y(), normal.z()),
                uv: Vec2::new(
                    uv.x() as f64 / skin.width as f64,
                    uv.y() as f64 / skin.height as f64,
                ),
            })
        }
        surface.indices.extend([base, base + 2, base + 1]);
    }
    let names = surfaces.iter().map(|s| s.texture.clone()).collect();
    Ok((
        surfaces
            .into_iter()
            .filter(|s| !s.indices.is_empty())
            .collect(),
        names,
    ))
}

fn generated_pbr(
    source: &map_model::model_pbr::ModelPbr,
) -> Result<scene_export::materials::GeneratedPbr, Error> {
    let mut images = Vec::new();
    for (slot, map) in [
        ("normal", &source.normal),
        ("mr", &source.metallic_roughness),
        ("ao", &source.occlusion),
        ("emissive", &source.emissive),
    ] {
        if let Some(map) = map {
            let godot_png = if slot == "ao" && source.occlusion_strength != 1. {
                let mut pixels = map.pixels.clone();
                for pixel in pixels.chunks_exact_mut(4) {
                    pixel[0] = (255.0 + (pixel[0] as f32 - 255.0) * source.occlusion_strength)
                        .round() as u8;
                }
                Some(
                    crate::assets::encode_png(map.width, map.height, &pixels)
                        .map_err(Error::Map)?
                        .into(),
                )
            } else {
                None
            };
            images.push(scene_export::materials::PbrImage {
                slot,
                godot_png,
                path: String::new(),
                png: crate::assets::encode_png(map.width, map.height, &map.pixels)
                    .map_err(Error::Map)?
                    .into(),
            });
        }
    }
    Ok(scene_export::materials::GeneratedPbr {
        alpha_mode: match source.alpha_mode {
            map_model::material::Alpha::Opaque => scene_export::materials::AlphaMode::Opaque,
            map_model::material::Alpha::Masked => scene_export::materials::AlphaMode::Mask,
            map_model::material::Alpha::Blended => scene_export::materials::AlphaMode::Blend,
        },
        alpha_cutoff: source.alpha_cutoff,
        double_sided: source.double_sided,
        roughness: source.roughness,
        metallic: source.metallic,
        emissive_factor: source.emissive_factor,
        normal_scale: source.normal_scale,
        occlusion_strength: source.occlusion_strength,
        images,
    })
}
