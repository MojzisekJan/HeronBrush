//! Shared static model geometry and content-verified mesh file reuse.
use crate::{
    geometry::{Mesh, SolidGeometry, Surface},
    level::Transform,
    map::Entity,
    math::{self, Vec3},
};

#[derive(Debug, Clone)]
pub struct ModelAsset {
    pub surfaces: std::sync::Arc<[Surface]>,
    /// Rotation columns in source map axes, before scale.
    pub rotation: [[f64; 3]; 3],
    pub scale: [f64; 3],
}

/// Keeps collision, animation, custom pivots and mirrored geometry on the baked path.
pub fn apply(
    entity: &Entity,
    geometry: &mut SolidGeometry,
    scale_factor: f64,
) -> Option<Transform> {
    let asset = entity.external_asset.as_ref()?;
    if geometry.mesh.is_empty()
        || !entity.brushes.is_empty()
        || !entity.patches.is_empty()
        || !geometry.concave_triangles.is_empty()
        || !geometry.convex_hulls.is_empty()
        || asset.scale.iter().any(|s| !s.is_finite() || *s <= 0.0)
        || entity.classname().starts_with("func_")
        || entity.classname().starts_with("trigger_")
        || entity
            .properties
            .keys()
            .any(|k| k.starts_with("animation_") || k.starts_with("motion_"))
    {
        return None;
    }
    let c = |p: [f64; 3]| math::to_godot(Vec3::from_array(p));
    let rotation = glam::DMat3::from_cols(
        c(asset.rotation[1]),
        c(asset.rotation[2]),
        c(asset.rotation[0]),
    );
    let (y, x, z) = glam::DQuat::from_mat3(&rotation).to_euler(glam::EulerRot::YXZ);
    let origin = entity.get_vec3("origin").unwrap_or(Vec3::ZERO);
    let mut mesh = Mesh::default();
    // Merge equal materials just as the normal geometry build does.
    for incoming in asset.surfaces.iter() {
        let index = mesh
            .surfaces
            .iter()
            .position(|s| s.texture == incoming.texture)
            .unwrap_or_else(|| {
                mesh.surfaces.push(Surface {
                    texture: incoming.texture.clone(),
                    vertices: vec![],
                    indices: vec![],
                });
                mesh.surfaces.len() - 1
            });
        let surface = &mut mesh.surfaces[index];
        let offset = surface.vertices.len() as u32;
        surface.vertices.extend(incoming.vertices.iter().map(|v| {
            let mut v = v.clone();
            v.position = math::to_godot(v.position) * scale_factor;
            v.normal = math::to_godot(v.normal);
            v
        }));
        surface
            .indices
            .extend(incoming.indices.iter().map(|i| i + offset));
    }
    geometry.mesh = mesh;
    geometry.origin = origin;
    Some(Transform {
        translation: (math::to_godot(origin) * scale_factor).to_array(),
        rotation_yxz: [x, y, z],
        scale: c(asset.scale).to_array(),
    })
}

/// Hashes only select candidates; full byte equality decides whether files are shared.
pub fn aliases<'a>(
    files: impl Iterator<Item = (&'a str, &'a [u8])>,
) -> std::collections::HashMap<String, String> {
    use std::{
        collections::HashMap,
        hash::{Hash, Hasher},
    };
    let mut buckets: HashMap<u64, Vec<(&str, &[u8])>> = HashMap::new();
    let mut aliases = HashMap::new();
    for (name, bytes) in files {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hash);
        let candidates = buckets.entry(hash.finish()).or_default();
        if let Some((original, _)) = candidates.iter().find(|(_, prior)| *prior == bytes) {
            aliases.insert(name.to_owned(), (*original).to_owned());
        } else {
            candidates.push((name, bytes));
        }
    }
    aliases
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collision_animation_and_mirrors_keep_baked_geometry() {
        use crate::geometry::Vertex;
        let surface = Surface {
            texture: "skin".into(),
            vertices: [Vec3::ZERO, Vec3::X, Vec3::Y]
                .map(|position| Vertex {
                    position,
                    normal: Vec3::Z,
                    uv: crate::math::Vec2::ZERO,
                })
                .to_vec(),
            indices: vec![0, 1, 2],
        };
        let mut entity = Entity::default();
        entity.properties.insert("classname".into(), "prop".into());
        entity.external_asset = Some(ModelAsset {
            surfaces: vec![surface.clone()].into(),
            rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            scale: [1.; 3],
        });
        for variant in 0..5 {
            let mut entity = entity.clone();
            let mut geometry = SolidGeometry {
                mesh: Mesh {
                    surfaces: vec![surface.clone()],
                },
                ..Default::default()
            };
            match variant {
                0 => geometry.concave_triangles = vec![Vec3::X, Vec3::Y, Vec3::Z],
                1 => geometry.convex_hulls = vec![vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::ZERO]],
                2 => {
                    entity
                        .properties
                        .insert("animation_keys".into(), "[]".into());
                }
                3 => entity.external_asset.as_mut().unwrap().scale[0] = -1.,
                _ => geometry.mesh = Mesh::default(),
            }
            let before = geometry.mesh.triangle_count();
            assert!(apply(&entity, &mut geometry, 1.).is_none());
            assert_eq!(geometry.mesh.triangle_count(), before);
        }
    }
    #[test]
    fn only_identical_file_bytes_are_aliased() {
        let aliases = aliases(
            [
                ("first", b"mesh".as_slice()),
                ("same", b"mesh".as_slice()),
                ("different", b"other".as_slice()),
            ]
            .into_iter(),
        );
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases.get("same").unwrap(), "first");
    }
}
