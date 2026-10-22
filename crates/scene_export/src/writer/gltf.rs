//! glTF 2.0 binary (`.glb`) writer.
//!
//! One file per solid entity, mirroring the `.obj` the Godot path writes, and
//! one primitive per texture so the engine can give each its own material.
//! A texture with more than 65,535 vertices is split over several primitives
//! sharing that material, so every visual index list fits in 16 bits.
//! Bevy reads glTF without a third-party crate; OBJ it does not.
//!
//! glTF and Bevy agree with Godot on axes and units, and vertices arrive here
//! already converted, so positions and normals go out untouched. Winding is
//! a different story: `geometry::Mesh` is wound clockwise seen from outside,
//! because that is what feeds the `.obj` writer, which counts on Godot's
//! importer to flip it back. glTF has no such importer flipping things back,
//! so this writer reverses the triangles itself, the same as the `.obj`
//! writer does, or Bevy's backface culling hides the wall facing the player
//! and shows the one behind it instead.

use crate::geometry::Mesh;
use crate::math::Vec3;
use indexmap::IndexMap;
use serde::Serialize;

const FLOAT: u32 = 5126;
const UNSIGNED_INT: u32 = 5125;
const UNSIGNED_SHORT: u32 = 5123;
const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;

#[derive(Debug, Clone, Copy, Default)]
pub struct GltfOptions {
    /// Write triangles clockwise instead, which turns every face inside out.
    pub flip_faces: bool,
    /// Measure V from the bottom of the image instead of the top.
    pub flip_uv_v: bool,
}

/// Collision geometry to carry alongside the visual mesh, in a glTF mesh of
/// its own. Positions arrive here already in engine space, the same as the
/// visual mesh. No winding, no normals: a physics engine reads it unoriented.
#[derive(Debug, Clone, Default)]
pub struct Collision {
    /// One point cloud per convex hull. Each becomes its own primitive.
    pub hulls: Vec<Vec<Vec3>>,
    /// A single concave triangle soup, three points to a triangle.
    pub triangles: Vec<Vec3>,
}

impl Collision {
    pub fn is_empty(&self) -> bool {
        self.hulls.is_empty() && self.triangles.is_empty()
    }

    /// `Trimesh` when there is a soup, `Convex` otherwise.
    pub fn kind(&self) -> crate::level::CollisionShape {
        if self.triangles.is_empty() {
            crate::level::CollisionShape::Convex
        } else {
            crate::level::CollisionShape::Trimesh
        }
    }

    /// How many primitives the written mesh holds: one for a soup, one per
    /// hull for a convex body.
    pub fn parts(&self) -> usize {
        if self.triangles.is_empty() {
            self.hulls.len()
        } else {
            1
        }
    }
}

/// A written file, plus the texture each primitive is waiting for.
#[derive(Debug, Clone)]
pub struct Glb {
    pub bytes: Vec<u8>,
    /// Texture names, in primitive order, for glTF mesh 0. Surfaces with no
    /// triangles are left out of the file, so this is not always the mesh's
    /// own surface list.
    pub textures: Vec<String>,
    /// The glTF mesh index that holds collision geometry, if any was written,
    /// and how many primitives it spread over.
    pub collision: Option<(usize, usize)>,
}

#[derive(Serialize)]
struct Asset {
    version: &'static str,
    generator: String,
}

#[derive(Serialize)]
struct BufferView {
    buffer: usize,
    #[serde(rename = "byteOffset")]
    byte_offset: usize,
    #[serde(rename = "byteLength")]
    byte_length: usize,
    target: u32,
}

#[derive(Serialize)]
struct Accessor {
    #[serde(rename = "bufferView")]
    buffer_view: usize,
    #[serde(rename = "componentType")]
    component_type: u32,
    count: usize,
    #[serde(rename = "type")]
    ty: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    min: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max: Option<Vec<f32>>,
}

#[derive(Serialize)]
struct Primitive {
    attributes: IndexMap<&'static str, usize>,
    indices: usize,
    /// Absent on collision primitives, which wear no material.
    #[serde(skip_serializing_if = "Option::is_none")]
    material: Option<usize>,
}

#[derive(Serialize)]
struct GltfMesh {
    name: String,
    primitives: Vec<Primitive>,
}

#[derive(Serialize)]
struct Node {
    name: String,
    mesh: usize,
}

#[derive(Serialize)]
struct Scene {
    nodes: Vec<usize>,
}

#[derive(Serialize)]
struct Material {
    name: String,
}

#[derive(Serialize)]
struct Buffer {
    #[serde(rename = "byteLength")]
    byte_length: usize,
}

#[derive(Serialize)]
struct Document {
    asset: Asset,
    // A collision-only `.glb` has no scene at all: its one mesh is reached by
    // index, never by walking a node tree.
    #[serde(skip_serializing_if = "Option::is_none")]
    scene: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    scenes: Vec<Scene>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    nodes: Vec<Node>,
    meshes: Vec<GltfMesh>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    materials: Vec<Material>,
    accessors: Vec<Accessor>,
    #[serde(rename = "bufferViews")]
    buffer_views: Vec<BufferView>,
    buffers: Vec<Buffer>,
}

/// Appends a buffer view over data just written, and hands back its index.
struct Blob {
    bytes: Vec<u8>,
    views: Vec<BufferView>,
}

impl Blob {
    fn new(room: usize) -> Blob {
        Blob {
            bytes: Vec::with_capacity(room),
            views: Vec::new(),
        }
    }

    fn push_floats(&mut self, values: &[f32]) -> usize {
        let start = self.bytes.len();
        for v in values {
            self.bytes.extend_from_slice(&v.to_le_bytes());
        }
        self.view(start, ARRAY_BUFFER)
    }

    /// Pushes an index list and returns its view and component type: 16-bit when every
    /// index fits, as raylib (even 6.0) truncates 32-bit glTF indices to 16 bits and
    /// most engines read the smaller form faster anyway.
    fn push_indices(&mut self, values: &[u32]) -> (usize, u32) {
        let start = self.bytes.len();
        // glTF reserves the type's maximum (65535) as the primitive-restart value,
        // so it must not appear in a 16-bit index list.
        let short = values.iter().all(|&v| v < u16::MAX as u32);
        for &v in values {
            if short {
                self.bytes.extend_from_slice(&(v as u16).to_le_bytes());
            } else {
                self.bytes.extend_from_slice(&v.to_le_bytes());
            }
        }
        let view = self.view(start, ELEMENT_ARRAY_BUFFER);
        // The next view holds floats, which glTF wants 4-byte aligned.
        while !self.bytes.len().is_multiple_of(4) {
            self.bytes.push(0);
        }
        (view, if short { UNSIGNED_SHORT } else { UNSIGNED_INT })
    }

    fn view(&mut self, start: usize, target: u32) -> usize {
        self.views.push(BufferView {
            buffer: 0,
            byte_offset: start,
            byte_length: self.bytes.len() - start,
            target,
        });
        self.views.len() - 1
    }
}

/// Most vertices one primitive may hold and still use 16-bit indices: 0..=65534,
/// since glTF forbids 65535 (the restart value) in an index list.
const MAX_SHORT_VERTICES: usize = u16::MAX as usize;

/// Splits a triangle list over `vertex_count` vertices into pieces that each
/// reference at most [`MAX_SHORT_VERTICES`] vertices. Each piece is the source
/// vertex ids it uses (None: all of them, in order, the usual single piece) and
/// its triangle list remapped onto those.
fn split_for_short_indices(
    vertex_count: usize,
    indices: Vec<u32>,
) -> Vec<(Option<Vec<u32>>, Vec<u32>)> {
    if vertex_count <= MAX_SHORT_VERTICES {
        return vec![(None, indices)];
    }
    let mut pieces = Vec::new();
    let mut local = vec![u32::MAX; vertex_count];
    let mut source: Vec<u32> = Vec::new();
    let mut remapped: Vec<u32> = Vec::new();
    for tri in indices.as_chunks::<3>().0 {
        let fresh = tri
            .iter()
            .filter(|&&v| local[v as usize] == u32::MAX)
            .count();
        if source.len() + fresh > MAX_SHORT_VERTICES {
            for &v in &source {
                local[v as usize] = u32::MAX;
            }
            pieces.push((
                Some(std::mem::take(&mut source)),
                std::mem::take(&mut remapped),
            ));
        }
        for &v in tri {
            if local[v as usize] == u32::MAX {
                local[v as usize] = source.len() as u32;
                source.push(v);
            }
            remapped.push(local[v as usize]);
        }
    }
    if !remapped.is_empty() {
        pieces.push((Some(source), remapped));
    }
    pieces
}

/// Writes one solid entity as a `.glb`: the visual mesh as glTF mesh 0, the
/// collision geometry, if any, as a mesh of its own. Returns None when there
/// is neither.
pub fn write(mesh: &Mesh, collision: &Collision, name: &str, options: GltfOptions) -> Option<Glb> {
    let room: usize = mesh
        .surfaces
        .iter()
        .map(|s| s.vertices.len() * 32 + s.indices.len() * 4)
        .sum();
    let mut blob = Blob::new(room);
    let mut accessors: Vec<Accessor> = Vec::new();
    let mut primitives: Vec<Primitive> = Vec::new();
    let mut materials: IndexMap<String, usize> = IndexMap::new();
    let mut textures: Vec<String> = Vec::new();

    for surface in &mesh.surfaces {
        if surface.indices.is_empty() || surface.vertices.is_empty() {
            continue;
        }

        let mut wound: Vec<u32> = Vec::with_capacity(surface.indices.len());
        for tri in surface.indices.chunks(3) {
            if tri.len() < 3 {
                continue;
            }
            // `geometry::Mesh` winds clockwise seen from outside (the `.obj`
            // writer relies on Godot's importer to flip it back); glTF has no
            // such importer, so the reversal has to happen here instead.
            let order = if options.flip_faces {
                [0, 1, 2]
            } else {
                [0, 2, 1]
            };
            for slot in order {
                wound.push(tri[slot]);
            }
        }
        if wound.is_empty() {
            continue;
        }

        // A surface too big for 16-bit indices goes out as several primitives with
        // the same texture. raylib (5.5 and 6.0) stores mesh indices as u16 and
        // silently truncates 32-bit glTF indices, which scrambles the geometry.
        for (source, indices) in split_for_short_indices(surface.vertices.len(), wound) {
            let count = source.as_ref().map_or(surface.vertices.len(), Vec::len);
            let mut positions: Vec<f32> = Vec::with_capacity(count * 3);
            let mut normals: Vec<f32> = Vec::with_capacity(count * 3);
            let mut uvs: Vec<f32> = Vec::with_capacity(count * 2);
            // glTF wants the bounds of the position accessor, and asking for them
            // after the cast to f32 keeps them exactly on the written values.
            let mut min = [f32::INFINITY; 3];
            let mut max = [f32::NEG_INFINITY; 3];

            let mut emit = |v: &crate::geometry::Vertex| {
                let p = [
                    v.position.x as f32,
                    v.position.y as f32,
                    v.position.z as f32,
                ];
                for i in 0..3 {
                    min[i] = min[i].min(p[i]);
                    max[i] = max[i].max(p[i]);
                }
                positions.extend_from_slice(&p);
                normals.extend_from_slice(&[
                    v.normal.x as f32,
                    v.normal.y as f32,
                    v.normal.z as f32,
                ]);
                let uv_v = if options.flip_uv_v {
                    1.0 - v.uv.y
                } else {
                    v.uv.y
                };
                uvs.extend_from_slice(&[v.uv.x as f32, uv_v as f32]);
            };
            match &source {
                None => surface.vertices.iter().for_each(&mut emit),
                Some(ids) => ids
                    .iter()
                    .for_each(|&i| emit(&surface.vertices[i as usize])),
            }

            let position_view = blob.push_floats(&positions);
            let normal_view = blob.push_floats(&normals);
            let uv_view = blob.push_floats(&uvs);
            let (index_view, index_type) = blob.push_indices(&indices);

            let base = accessors.len();
            accessors.push(Accessor {
                buffer_view: position_view,
                component_type: FLOAT,
                count,
                ty: "VEC3",
                min: Some(min.to_vec()),
                max: Some(max.to_vec()),
            });
            accessors.push(Accessor {
                buffer_view: normal_view,
                component_type: FLOAT,
                count,
                ty: "VEC3",
                min: None,
                max: None,
            });
            accessors.push(Accessor {
                buffer_view: uv_view,
                component_type: FLOAT,
                count,
                ty: "VEC2",
                min: None,
                max: None,
            });
            accessors.push(Accessor {
                buffer_view: index_view,
                component_type: index_type,
                count: indices.len(),
                ty: "SCALAR",
                min: None,
                max: None,
            });

            let next = materials.len();
            let material = *materials.entry(surface.texture.clone()).or_insert(next);

            let mut attributes = IndexMap::new();
            attributes.insert("POSITION", base);
            attributes.insert("NORMAL", base + 1);
            attributes.insert("TEXCOORD_0", base + 2);
            primitives.push(Primitive {
                attributes,
                indices: base + 3,
                material: Some(material),
            });
            textures.push(surface.texture.clone());
        }
    }

    // glTF mesh 0 is the visual mesh, when there is one. A solid drawn only in
    // `clip` has none, and its collision mesh takes index 0 instead.
    let mut meshes: Vec<GltfMesh> = Vec::new();
    let has_visual = !primitives.is_empty();
    if has_visual {
        meshes.push(GltfMesh {
            name: name.to_string(),
            primitives,
        });
    }

    let collision_info = if collision.is_empty() {
        None
    } else {
        let mut parts: Vec<Primitive> = Vec::new();
        for hull in &collision.hulls {
            if hull.len() < 3 {
                continue;
            }
            parts.push(position_primitive(
                &mut blob,
                &mut accessors,
                hull,
                &fan_indices(hull.len()),
            ));
        }
        if !collision.triangles.is_empty() {
            let soup: Vec<u32> = (0..collision.triangles.len() as u32).collect();
            parts.push(position_primitive(
                &mut blob,
                &mut accessors,
                &collision.triangles,
                &soup,
            ));
        }
        if parts.is_empty() {
            None
        } else {
            let index = meshes.len();
            let count = parts.len();
            meshes.push(GltfMesh {
                name: format!("{name}_collision"),
                primitives: parts,
            });
            Some((index, count))
        }
    };

    if meshes.is_empty() {
        return None;
    }

    // The scene holds one node pointing at the visual mesh. The collision
    // mesh, when there is one, is left out of the scene, so drawing the glTF
    // never draws it. A collision-only file has no scene at all.
    let (scene, scenes, nodes) = if has_visual {
        (
            Some(0),
            vec![Scene { nodes: vec![0] }],
            vec![Node {
                name: name.to_string(),
                mesh: 0,
            }],
        )
    } else {
        (None, Vec::new(), Vec::new())
    };

    let document = Document {
        asset: Asset {
            version: "2.0",
            generator: format!("heron_brush {}", env!("CARGO_PKG_VERSION")),
        },
        scene,
        scenes,
        nodes,
        meshes,
        // Named after the texture, and otherwise left at the glTF default. The
        // engine side has the level file, which says where the image is.
        materials: materials
            .into_keys()
            .map(|name| Material { name })
            .collect(),
        accessors,
        buffer_views: blob.views,
        buffers: vec![Buffer {
            byte_length: blob.bytes.len(),
        }],
    };

    let json = serde_json::to_vec(&document).ok()?;
    Some(Glb {
        bytes: pack(&json, &blob.bytes),
        textures,
        collision: collision_info,
    })
}

/// A `POSITION`-only primitive over `points`, indexed by `indices`, with no
/// material. Used for collision geometry, which needs nothing else.
fn position_primitive(
    blob: &mut Blob,
    accessors: &mut Vec<Accessor>,
    points: &[Vec3],
    indices: &[u32],
) -> Primitive {
    let mut flat: Vec<f32> = Vec::with_capacity(points.len() * 3);
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in points {
        let v = [p.x as f32, p.y as f32, p.z as f32];
        for i in 0..3 {
            min[i] = min[i].min(v[i]);
            max[i] = max[i].max(v[i]);
        }
        flat.extend_from_slice(&v);
    }
    let position_view = blob.push_floats(&flat);
    let (index_view, index_type) = blob.push_indices(indices);
    let base = accessors.len();
    accessors.push(Accessor {
        buffer_view: position_view,
        component_type: FLOAT,
        count: points.len(),
        ty: "VEC3",
        min: Some(min.to_vec()),
        max: Some(max.to_vec()),
    });
    accessors.push(Accessor {
        buffer_view: index_view,
        component_type: index_type,
        count: indices.len(),
        ty: "SCALAR",
        min: None,
        max: None,
    });
    let mut attributes = IndexMap::new();
    attributes.insert("POSITION", base);
    Primitive {
        attributes,
        indices: base + 1,
        material: None,
    }
}

/// Triangle-fan indices over `n` points. The fan is not a real surface; a
/// physics engine takes the point set and ignores the topology.
fn fan_indices(n: usize) -> Vec<u32> {
    let mut out = Vec::with_capacity(n.saturating_sub(2) * 3);
    for i in 1..n.saturating_sub(1) {
        out.extend_from_slice(&[0, i as u32, i as u32 + 1]);
    }
    out
}

/// Portable base-color image resolved once for an entire level export, with the
/// material's normal map when the editor supplied one.
#[derive(Clone)]
pub(crate) struct AlbedoImage {
    bytes: std::sync::Arc<[u8]>,
    mime: &'static str,
    clamp: bool,
    alpha: bool,
    surface: Option<[f32; 3]>,
    /// PNG bytes in the glTF convention, see `Settings::normal_maps`.
    normal: Option<std::sync::Arc<[u8]>>,
    pbr: Option<crate::materials::GeneratedPbr>,
}

/// Load ordinary albedos once per path, preserving external custom materials.
/// Only core glTF PNG and JPEG image formats are embedded. Other texture
/// formats stay external and produce a warning instead of a false MIME type.
pub(crate) fn prepare_albedos(
    library: &crate::materials::MaterialLibrary,
    settings: &crate::settings::Settings,
) -> (IndexMap<String, AlbedoImage>, Vec<String>) {
    use crate::materials::{res_to_fs, Material};
    let mut result = IndexMap::new();
    let mut warnings = Vec::new();
    let mut files = std::collections::HashMap::<std::path::PathBuf, Option<AlbedoImage>>::new();
    for (name, material) in &library.materials {
        let Material::FromTextures { albedo, .. } = material else {
            continue;
        };
        let normal = settings.normal_maps.get(name).cloned();
        if let Some(image) = library.generated.get(name) {
            result.insert(
                name.clone(),
                AlbedoImage {
                    bytes: image.png.clone().into(),
                    mime: "image/png",
                    clamp: !image.repeat,
                    alpha: image.alpha,
                    surface: image.surface,
                    normal,
                    pbr: image.pbr.clone(),
                },
            );
            continue;
        }
        let Some(root) = settings.project_root.as_deref() else {
            continue;
        };
        let path = res_to_fs(root, albedo);
        let image = files.entry(path.clone()).or_insert_with(|| {
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    warnings.push(format!("could not embed albedo `{}` in GLB: {error}", path.display()));
                    return None;
                }
            };
            let Some((mime, alpha)) = image_format(&bytes) else {
                warnings.push(format!("albedo `{}` was not embedded in GLB: core glTF requires PNG or JPEG; convert this image or assign a native material", path.display()));
                return None;
            };
            Some(AlbedoImage { bytes: bytes.into(), mime, clamp: false, alpha, surface: None, normal: None, pbr: None })
        });
        if let Some(image) = image {
            result.insert(
                name.clone(),
                AlbedoImage {
                    normal,
                    ..image.clone()
                },
            );
        }
    }
    (result, warnings)
}

fn image_format(bytes: &[u8]) -> Option<(&'static str, bool)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().ok()?;
        let mut decoded = vec![0; reader.output_buffer_size()?];
        let info = reader.next_frame(&mut decoded).ok()?;
        let pixels = &decoded[..info.buffer_size()];
        let alpha = match info.color_type {
            png::ColorType::Rgba => pixels.as_chunks::<4>().0.iter().any(|p| p[3] != 255),
            png::ColorType::GrayscaleAlpha => pixels.as_chunks::<2>().0.iter().any(|p| p[1] != 255),
            _ => false,
        };
        Some(("image/png", alpha))
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(("image/jpeg", false))
    } else {
        None
    }
}

/// Per-vertex glTF tangents (xyz unit, w the bitangent sign) for an indexed
/// triangle list, from the UV gradients of the triangles around each vertex.
///
/// glTF's normal texture is +Y up in the image, and with glTF UVs (V growing
/// downwards) image-up is -V; `flip_v` says the UVs were written V-up instead.
/// The bitangent a reader rebuilds as `cross(normal, tangent.xyz) * w` then
/// points image-up, the frame MikkTSpace-based tools produce for flat brush
/// faces. A vertex with no usable UV gradient gets any unit perpendicular.
pub fn tangents(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    indices: &[u32],
    flip_v: bool,
) -> Vec<[f32; 4]> {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let n = positions.len();
    let mut along_u = vec![[0.0f32; 3]; n];
    let mut along_v = vec![[0.0f32; 3]; n];
    for tri in indices.as_chunks::<3>().0 {
        let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        if a >= n || b >= n || c >= n || a >= uvs.len() || b >= uvs.len() || c >= uvs.len() {
            continue;
        }
        let (e1, e2) = (
            sub(positions[b], positions[a]),
            sub(positions[c], positions[a]),
        );
        let (du1, dv1) = (uvs[b][0] - uvs[a][0], uvs[b][1] - uvs[a][1]);
        let (du2, dv2) = (uvs[c][0] - uvs[a][0], uvs[c][1] - uvs[a][1]);
        let det = du1 * dv2 - du2 * dv1;
        if det.abs() < 1e-12 {
            continue;
        }
        let r = 1.0 / det;
        let u: [f32; 3] = std::array::from_fn(|i| (e1[i] * dv2 - e2[i] * dv1) * r);
        let v: [f32; 3] = std::array::from_fn(|i| (e2[i] * du1 - e1[i] * du2) * r);
        for i in [a, b, c] {
            for k in 0..3 {
                along_u[i][k] += u[k];
                along_v[i][k] += v[k];
            }
        }
    }
    (0..n)
        .map(|i| {
            let normal = normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]);
            let mut t = sub(along_u[i], normal.map(|c| c * dot(normal, along_u[i])));
            let mut length = dot(t, t).sqrt();
            if length < 1e-12 {
                // No gradient: any direction perpendicular to the normal.
                let other = if normal[0].abs() < 0.9 {
                    [1.0, 0.0, 0.0]
                } else {
                    [0.0, 1.0, 0.0]
                };
                t = cross(other, normal);
                length = dot(t, t).sqrt().max(1e-12);
            }
            let t = t.map(|c| c / length);
            let up = if flip_v {
                along_v[i]
            } else {
                along_v[i].map(|c| -c)
            };
            let w = if dot(cross(normal, t), up) < 0.0 {
                -1.0
            } else {
                1.0
            };
            [t[0], t[1], t[2], w]
        })
        .collect()
}

/// Reads accessor `index` of `doc` out of `binary` as `width` floats per element
/// (or as indices, for a SCALAR unsigned accessor). Only the tightly packed
/// accessors this writer produces are understood.
fn read_accessor(
    doc: &serde_json::Value,
    binary: &[u8],
    index: usize,
) -> Option<(Vec<f32>, Vec<u32>)> {
    let accessor = &doc["accessors"][index];
    let view = &doc["bufferViews"][accessor["bufferView"].as_u64()? as usize];
    let start = view["byteOffset"].as_u64().unwrap_or(0) as usize
        + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let count = accessor["count"].as_u64()? as usize;
    let width = match accessor["type"].as_str()? {
        "SCALAR" => 1,
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" => 4,
        _ => return None,
    };
    let component = accessor["componentType"].as_u64()? as u32;
    let size = if component == UNSIGNED_SHORT { 2 } else { 4 };
    let bytes = binary.get(start..start + count * width * size)?;
    Some(match component {
        FLOAT => (
            bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect(),
            Vec::new(),
        ),
        UNSIGNED_INT => (
            Vec::new(),
            bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| u32::from_le_bytes(*b))
                .collect(),
        ),
        UNSIGNED_SHORT => (
            Vec::new(),
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes(*b) as u32)
                .collect(),
        ),
        _ => return None,
    })
}

/// Embed resolved albedos, and the normal maps beside them, so native importers
/// need no custom level loader. The source GLB is our own geometry-only writer
/// output, with no images yet. A primitive whose material gains a normal map
/// also gains a `TANGENT` attribute; `flip_uv_v` is the writer's V convention.
pub(crate) fn embed_materials(
    glb: &mut Glb,
    albedos: &IndexMap<String, AlbedoImage>,
    flip_uv_v: bool,
) {
    if albedos.is_empty() {
        return;
    }
    let json_size = u32::from_le_bytes(glb.bytes[12..16].try_into().unwrap()) as usize;
    let mut doc: serde_json::Value =
        serde_json::from_slice(&glb.bytes[20..20 + json_size]).unwrap();
    let binary_start = 20 + json_size + 8;
    let original_size = doc["buffers"][0]["byteLength"].as_u64().unwrap() as usize;
    let mut binary = glb.bytes[binary_start..binary_start + original_size].to_vec();
    let mut images = Vec::new();
    let mut textures = Vec::new();
    let mut image_indices = std::collections::HashMap::<std::sync::Arc<[u8]>, usize>::new();
    let mut texture_indices = std::collections::HashMap::<(usize, bool), usize>::new();
    let mut normal_mapped = std::collections::HashSet::<usize>::new();
    let mut texture = |bytes: &std::sync::Arc<[u8]>,
                       mime: &'static str,
                       clamp: bool,
                       doc: &mut serde_json::Value,
                       binary: &mut Vec<u8>| {
        let image_index = *image_indices.entry(bytes.clone()).or_insert_with(|| {
            while !binary.len().is_multiple_of(4) {
                binary.push(0);
            }
            let offset = binary.len();
            binary.extend_from_slice(bytes);
            let views = doc["bufferViews"].as_array_mut().unwrap();
            let view = views.len();
            views
                .push(serde_json::json!({"buffer":0,"byteOffset":offset,"byteLength":bytes.len()}));
            let image_index = images.len();
            images.push(serde_json::json!({"bufferView":view,"mimeType":mime}));
            image_index
        });
        *texture_indices
            .entry((image_index, clamp))
            .or_insert_with(|| {
                let texture_index = textures.len();
                textures
                    .push(serde_json::json!({"source":image_index,"sampler":usize::from(clamp)}));
                texture_index
            })
    };
    for index in 0..doc["materials"].as_array().map_or(0, |v| v.len()) {
        let Some(name) = doc["materials"][index]["name"].as_str() else {
            continue;
        };
        let Some(image) = albedos.get(name) else {
            continue;
        };
        let texture_index = texture(&image.bytes, image.mime, image.clamp, &mut doc, &mut binary);
        doc["materials"][index]["pbrMetallicRoughness"] = serde_json::json!({
            "baseColorTexture":{"index":texture_index},"metallicFactor":image.surface.unwrap_or([1.0,0.0,0.0])[1],"roughnessFactor":image.surface.unwrap_or([1.0,0.0,0.0])[0]
        });
        let emission = image.surface.map_or(0.0, |s| s[2]);
        if emission > 0.0 {
            doc["materials"][index]["emissiveTexture"] = serde_json::json!({"index":texture_index});
            doc["materials"][index]["emissiveFactor"] = serde_json::json!([1.0, 1.0, 1.0]);
            doc["materials"][index]["extensions"]["KHR_materials_emissive_strength"] =
                serde_json::json!({"emissiveStrength":emission});
            doc["extensionsUsed"] = serde_json::json!(["KHR_materials_emissive_strength"]);
        }
        if image.alpha {
            doc["materials"][index]["alphaMode"] = serde_json::json!("BLEND");
        }
        if let Some(normal) = &image.normal {
            let normal_index = texture(normal, "image/png", image.clamp, &mut doc, &mut binary);
            doc["materials"][index]["normalTexture"] = serde_json::json!({"index":normal_index});
            normal_mapped.insert(index);
        }
        if let Some(pbr) = &image.pbr {
            let mat = &mut doc["materials"][index];
            mat["pbrMetallicRoughness"]["roughnessFactor"] = serde_json::json!(pbr.roughness);
            mat["pbrMetallicRoughness"]["metallicFactor"] = serde_json::json!(pbr.metallic);
            mat["emissiveFactor"] = serde_json::json!(pbr.emissive_factor);
            // Pixel alpha alone cannot distinguish opaque, cutout and blended
            // source materials. Explicit imported rules override that fallback.
            mat["alphaMode"] = serde_json::json!(match pbr.alpha_mode {
                crate::materials::AlphaMode::Opaque => "OPAQUE",
                crate::materials::AlphaMode::Mask => "MASK",
                crate::materials::AlphaMode::Blend => "BLEND",
            });
            if pbr.alpha_mode == crate::materials::AlphaMode::Mask {
                mat["alphaCutoff"] = serde_json::json!(pbr.alpha_cutoff);
            }
            mat["doubleSided"] = serde_json::json!(pbr.double_sided);
            for map in &pbr.images {
                let id = texture(&map.png, "image/png", image.clamp, &mut doc, &mut binary);
                let mat = &mut doc["materials"][index];
                match map.slot {
                    "normal" => {
                        mat["normalTexture"] =
                            serde_json::json!({"index":id,"scale":pbr.normal_scale});
                        normal_mapped.insert(index);
                    }
                    "mr" => {
                        mat["pbrMetallicRoughness"]["metallicRoughnessTexture"] =
                            serde_json::json!({"index":id})
                    }
                    "ao" => {
                        mat["occlusionTexture"] =
                            serde_json::json!({"index":id,"strength":pbr.occlusion_strength})
                    }
                    "emissive" => mat["emissiveTexture"] = serde_json::json!({"index":id}),
                    _ => {}
                }
            }
        }
    }
    if images.is_empty() {
        return;
    }

    // Tangents for every primitive wearing a normal-mapped material.
    let mut added = Vec::new();
    for (mesh, primitives) in doc["meshes"].as_array().into_iter().flatten().enumerate() {
        for (p, primitive) in primitives["primitives"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            let wears_normal = primitive["material"]
                .as_u64()
                .is_some_and(|m| normal_mapped.contains(&(m as usize)));
            if !wears_normal || primitive["attributes"].get("TANGENT").is_some() {
                continue;
            }
            let attribute = |key: &str| primitive["attributes"][key].as_u64().map(|i| i as usize);
            let (Some(position), Some(normal), Some(uv), Some(indices)) = (
                attribute("POSITION"),
                attribute("NORMAL"),
                attribute("TEXCOORD_0"),
                primitive["indices"].as_u64().map(|i| i as usize),
            ) else {
                continue;
            };
            let (Some((positions, _)), Some((normals, _)), Some((uvs, _)), Some((_, indices))) = (
                read_accessor(&doc, &binary, position),
                read_accessor(&doc, &binary, normal),
                read_accessor(&doc, &binary, uv),
                read_accessor(&doc, &binary, indices),
            ) else {
                continue;
            };
            let tangents = tangents(
                positions.as_chunks::<3>().0,
                normals.as_chunks::<3>().0,
                uvs.as_chunks::<2>().0,
                &indices,
                flip_uv_v,
            );
            added.push((mesh, p, tangents));
        }
    }
    for (mesh, p, tangents) in added {
        while !binary.len().is_multiple_of(4) {
            binary.push(0);
        }
        let offset = binary.len();
        for t in &tangents {
            for c in t {
                binary.extend_from_slice(&c.to_le_bytes());
            }
        }
        let views = doc["bufferViews"].as_array_mut().unwrap();
        views.push(serde_json::json!({"buffer":0,"byteOffset":offset,"byteLength":tangents.len()*16,"target":ARRAY_BUFFER}));
        let view = views.len() - 1;
        let accessors = doc["accessors"].as_array_mut().unwrap();
        accessors.push(serde_json::json!({"bufferView":view,"componentType":FLOAT,"count":tangents.len(),"type":"VEC4"}));
        let accessor = accessors.len() - 1;
        doc["meshes"][mesh]["primitives"][p]["attributes"]["TANGENT"] = serde_json::json!(accessor);
    }

    doc["images"] = serde_json::json!(images);
    doc["textures"] = serde_json::json!(textures);
    doc["samplers"] = serde_json::json!([
        {"wrapS":10497,"wrapT":10497,"magFilter":9729,"minFilter":9729},
        {"wrapS":33071,"wrapT":33071,"magFilter":9729,"minFilter":9729}
    ]);
    doc["buffers"][0]["byteLength"] = serde_json::json!(binary.len());
    glb.bytes = pack(&serde_json::to_vec(&doc).unwrap(), &binary);
}

/// Wraps the JSON and the binary blob in the GLB container. Both chunks are
/// padded to four bytes, the JSON one with spaces and the binary one with
/// zeros, which is what the specification asks for.
fn pack(json: &[u8], bin: &[u8]) -> Vec<u8> {
    let json_padding = (4 - json.len() % 4) % 4;
    let bin_padding = (4 - bin.len() % 4) % 4;
    let total = 12 + 8 + json.len() + json_padding + 8 + bin.len() + bin_padding;

    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());

    out.extend_from_slice(&((json.len() + json_padding) as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(json);
    out.extend(std::iter::repeat_n(b' ', json_padding));

    out.extend_from_slice(&((bin.len() + bin_padding) as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(bin);
    out.extend(std::iter::repeat_n(0u8, bin_padding));

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Surface, Vertex};
    use crate::math::{Vec2, Vec3};

    fn quad() -> Mesh {
        let vertices = vec![
            Vertex {
                position: Vec3::new(0.0, 0.0, 0.0),
                normal: Vec3::Y,
                uv: Vec2::new(0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(2.0, 0.0, 0.0),
                normal: Vec3::Y,
                uv: Vec2::new(1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(2.0, 0.0, 3.0),
                normal: Vec3::Y,
                uv: Vec2::new(1.0, 0.25),
            },
        ];
        Mesh {
            surfaces: vec![Surface {
                texture: "stone".into(),
                vertices,
                indices: vec![0, 1, 2],
            }],
        }
    }

    fn json_of(glb: &Glb) -> serde_json::Value {
        let length = u32::from_le_bytes(glb.bytes[12..16].try_into().unwrap()) as usize;
        serde_json::from_slice(&glb.bytes[20..20 + length]).unwrap()
    }

    /// The quad, no collision, default options: what most of these tests want.
    fn plain(mesh: &Mesh) -> Glb {
        write(mesh, &Collision::default(), "wall", GltfOptions::default()).unwrap()
    }

    #[test]
    fn the_container_says_gltf_and_states_its_own_length() {
        let glb = plain(&quad());
        assert_eq!(&glb.bytes[0..4], b"glTF");
        assert_eq!(u32::from_le_bytes(glb.bytes[4..8].try_into().unwrap()), 2);
        assert_eq!(
            u32::from_le_bytes(glb.bytes[8..12].try_into().unwrap()) as usize,
            glb.bytes.len()
        );
    }

    #[test]
    fn both_chunks_land_on_a_four_byte_boundary() {
        let glb = plain(&quad());
        assert_eq!(glb.bytes.len() % 4, 0);
        let json_length = u32::from_le_bytes(glb.bytes[12..16].try_into().unwrap()) as usize;
        assert_eq!(json_length % 4, 0);
        assert_eq!(&glb.bytes[16..20], b"JSON");
        assert_eq!(
            &glb.bytes[20 + json_length + 4..20 + json_length + 8],
            b"BIN\0"
        );
    }

    #[test]
    fn one_primitive_per_surface_each_with_its_own_material() {
        let mut mesh = quad();
        let mut second = mesh.surfaces[0].clone();
        second.texture = "wood".into();
        mesh.surfaces.push(second);
        let glb = plain(&mesh);

        assert_eq!(glb.textures, vec!["stone", "wood"]);
        let doc = json_of(&glb);
        assert_eq!(doc["meshes"][0]["primitives"].as_array().unwrap().len(), 2);
        assert_eq!(doc["materials"][0]["name"], "stone");
        assert_eq!(doc["materials"][1]["name"], "wood");
    }

    #[test]
    fn two_surfaces_sharing_a_texture_share_one_material() {
        let mut mesh = quad();
        let same = mesh.surfaces[0].clone();
        mesh.surfaces.push(same);
        let doc = json_of(&plain(&mesh));
        assert_eq!(doc["materials"].as_array().unwrap().len(), 1);
        assert_eq!(doc["meshes"][0]["primitives"][1]["material"], 0);
    }

    #[test]
    fn the_position_accessor_carries_the_bounds_gltf_asks_for() {
        let doc = json_of(&plain(&quad()));
        let position = &doc["accessors"][0];
        assert_eq!(position["type"], "VEC3");
        assert_eq!(position["count"], 3);
        assert_eq!(position["min"].as_array().unwrap()[0], 0.0);
        assert_eq!(position["max"].as_array().unwrap()[0], 2.0);
        assert_eq!(position["max"].as_array().unwrap()[2], 3.0);
    }

    #[test]
    fn index_65535_is_the_restart_value_so_it_forces_32_bits() {
        let mut blob = Blob::new(0);
        assert_eq!(blob.push_indices(&[0, 65_534, 1]).1, UNSIGNED_SHORT);
        assert_eq!(blob.push_indices(&[0, 65_535, 1]).1, UNSIGNED_INT);
    }

    #[test]
    fn a_surface_past_65535_vertices_splits_into_16_bit_primitives_of_one_texture() {
        // 25,000 unshared triangles: 75,000 vertices, more than u16 can address.
        // raylib 5.5/6.0 truncates 32-bit indices, so this must not be one primitive.
        let triangles = 25_000u32;
        let vertices: Vec<Vertex> = (0..triangles * 3)
            .map(|i| Vertex {
                position: Vec3::new(i as f64, (i % 3) as f64, 0.0),
                normal: Vec3::Y,
                uv: Vec2::new(0.0, 0.0),
            })
            .collect();
        let mesh = Mesh {
            surfaces: vec![Surface {
                texture: "big".into(),
                vertices,
                indices: (0..triangles * 3).collect(),
            }],
        };
        let glb = plain(&mesh);
        let json = json_of(&glb);
        let primitives = json["meshes"][0]["primitives"].as_array().unwrap();
        assert_eq!(primitives.len(), 2);
        assert_eq!(
            glb.textures,
            vec!["big", "big"],
            "one surface entry per primitive"
        );
        assert_eq!(
            json["materials"].as_array().unwrap().len(),
            1,
            "pieces share the material"
        );

        let json_length = u32::from_le_bytes(glb.bytes[12..16].try_into().unwrap()) as usize;
        let bin = &glb.bytes[20 + json_length + 8..];
        let mut drawn = 0;
        for primitive in primitives {
            let indices = &json["accessors"][primitive["indices"].as_u64().unwrap() as usize];
            let positions =
                &json["accessors"][primitive["attributes"]["POSITION"].as_u64().unwrap() as usize];
            assert_eq!(indices["componentType"], UNSIGNED_SHORT);
            let vertex_count = positions["count"].as_u64().unwrap() as usize;
            assert!(vertex_count <= MAX_SHORT_VERTICES);
            let view = &json["bufferViews"][indices["bufferView"].as_u64().unwrap() as usize];
            let offset = view["byteOffset"].as_u64().unwrap() as usize;
            let count = indices["count"].as_u64().unwrap() as usize;
            let position_offset = json["bufferViews"]
                [positions["bufferView"].as_u64().unwrap() as usize]["byteOffset"]
                .as_u64()
                .unwrap() as usize;
            assert_eq!(position_offset % 4, 0);
            for t in 0..count / 3 {
                let corner = |k: usize| {
                    let at = offset + (t * 3 + k) * 2;
                    let local = u16::from_le_bytes(bin[at..at + 2].try_into().unwrap()) as usize;
                    assert!(local < vertex_count);
                    let x = position_offset + local * 12;
                    f32::from_le_bytes(bin[x..x + 4].try_into().unwrap()) as u32
                };
                // Unshared triangle n is vertices 3n, 3n+1, 3n+2, wound 0-2-1.
                let (a, b, c) = (corner(0), corner(1), corner(2));
                assert_eq!((a % 3, b % 3, c % 3), (0, 2, 1));
                assert_eq!((b - a, c - a), (2, 1));
            }
            drawn += count / 3;
        }
        assert_eq!(drawn, triangles as usize, "no triangle lost or duplicated");
    }

    #[test]
    fn splitting_keeps_shared_vertices_shared_within_a_piece() {
        // A strip of 70,000 vertices with neighbouring triangles sharing an edge.
        let n = 70_000u32;
        let indices: Vec<u32> = (0..n - 2).flat_map(|i| [i, i + 1, i + 2]).collect();
        let pieces = split_for_short_indices(n as usize, indices.clone());
        assert_eq!(pieces.len(), 2);
        let mut rebuilt = Vec::new();
        for (source, local) in &pieces {
            let source = source.as_ref().unwrap();
            assert!(source.len() <= MAX_SHORT_VERTICES);
            assert!(local.iter().all(|&l| (l as usize) < source.len()));
            rebuilt.extend(local.iter().map(|&l| source[l as usize]));
        }
        assert_eq!(rebuilt, indices);
        assert!(matches!(
            split_for_short_indices(3, vec![0, 1, 2]).as_slice(),
            [(None, _)]
        ));
    }

    #[test]
    fn indices_past_u16_use_32_bits_and_views_stay_aligned() {
        let mut blob = Blob::new(0);
        let (_, small) = blob.push_indices(&[0, 1, 2]);
        assert_eq!(small, UNSIGNED_SHORT);
        assert_eq!(
            blob.bytes.len() % 4,
            0,
            "the next float view must start 4-byte aligned"
        );
        let (_, large) = blob.push_indices(&[0, 70_000, 2]);
        assert_eq!(large, UNSIGNED_INT);
    }

    #[test]
    fn winding_is_reversed_because_the_mesh_arrives_wound_for_obj_not_gltf() {
        let glb = plain(&quad());
        let doc = json_of(&glb);
        let view = doc["accessors"][3]["bufferView"].as_u64().unwrap() as usize;
        let offset = doc["bufferViews"][view]["byteOffset"].as_u64().unwrap() as usize;
        let json_length = u32::from_le_bytes(glb.bytes[12..16].try_into().unwrap()) as usize;
        let bin = &glb.bytes[20 + json_length + 8..];
        assert_eq!(
            doc["accessors"][3]["componentType"], UNSIGNED_SHORT,
            "small meshes use 16-bit indices"
        );
        let read = |i: usize| {
            u16::from_le_bytes(bin[offset + i * 2..offset + i * 2 + 2].try_into().unwrap())
        };
        assert_eq!([read(0), read(1), read(2)], [0, 2, 1]);

        let flipped = write(
            &quad(),
            &Collision::default(),
            "wall",
            GltfOptions {
                flip_faces: true,
                flip_uv_v: false,
            },
        )
        .unwrap();
        let flipped_bin = &flipped.bytes
            [20 + u32::from_le_bytes(flipped.bytes[12..16].try_into().unwrap()) as usize + 8..];
        let read = |i: usize| {
            u16::from_le_bytes(
                flipped_bin[offset + i * 2..offset + i * 2 + 2]
                    .try_into()
                    .unwrap(),
            )
        };
        assert_eq!([read(0), read(1), read(2)], [0, 1, 2]);
    }

    #[test]
    fn a_cube_comes_out_wound_counter_clockwise_seen_from_outside() {
        // The glTF and Bevy convention: a triangle is front-facing when its
        // vertices run counter-clockwise as seen from the side its normal
        // points to. `geometry::Mesh` winds the other way round, so this
        // checks the writer actually undoes that rather than passing it
        // through.
        let mut mesh = quad();
        mesh.surfaces[0].vertices[0].normal = crate::math::Vec3::Y;
        mesh.surfaces[0].vertices[1].normal = crate::math::Vec3::Y;
        mesh.surfaces[0].vertices[2].normal = crate::math::Vec3::Y;
        let glb = plain(&mesh);
        let doc = json_of(&glb);
        let json_length = u32::from_le_bytes(glb.bytes[12..16].try_into().unwrap()) as usize;
        let bin = &glb.bytes[20 + json_length + 8..];

        let position_view = doc["accessors"][0]["bufferView"].as_u64().unwrap() as usize;
        let position_offset = doc["bufferViews"][position_view]["byteOffset"]
            .as_u64()
            .unwrap() as usize;
        let read_vertex = |i: usize| -> crate::math::Vec3 {
            let at = position_offset + i * 12;
            let read_f32 = |j: usize| {
                f32::from_le_bytes(bin[at + j * 4..at + j * 4 + 4].try_into().unwrap()) as f64
            };
            crate::math::Vec3::new(read_f32(0), read_f32(1), read_f32(2))
        };

        let index_view = doc["accessors"][3]["bufferView"].as_u64().unwrap() as usize;
        let index_offset = doc["bufferViews"][index_view]["byteOffset"]
            .as_u64()
            .unwrap() as usize;
        let read_index = |i: usize| {
            u16::from_le_bytes(
                bin[index_offset + i * 2..index_offset + i * 2 + 2]
                    .try_into()
                    .unwrap(),
            ) as usize
        };

        let (a, b, c) = (
            read_vertex(read_index(0)),
            read_vertex(read_index(1)),
            read_vertex(read_index(2)),
        );
        let geometric = (b - a).cross(c - a);
        assert!(
            geometric.dot(crate::math::Vec3::Y) > 0.0,
            "expected counter-clockwise winding around the outward normal"
        );
    }

    #[test]
    fn v_is_measured_from_the_top_unless_asked_otherwise() {
        let glb = plain(&quad());
        let doc = json_of(&glb);
        let view = doc["accessors"][2]["bufferView"].as_u64().unwrap() as usize;
        let offset = doc["bufferViews"][view]["byteOffset"].as_u64().unwrap() as usize;
        let json_length = u32::from_le_bytes(glb.bytes[12..16].try_into().unwrap()) as usize;
        let bin = &glb.bytes[20 + json_length + 8..];
        // Third vertex, V component: the 0.25 the mesh carries, untouched.
        let at = offset + 5 * 4;
        assert_eq!(
            f32::from_le_bytes(bin[at..at + 4].try_into().unwrap()),
            0.25
        );
    }

    #[test]
    fn a_mesh_with_nothing_to_draw_writes_no_file() {
        let mut mesh = quad();
        mesh.surfaces[0].indices.clear();
        assert!(write(&mesh, &Collision::default(), "wall", GltfOptions::default()).is_none());
    }

    fn box_points() -> Vec<Vec3> {
        vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ]
    }

    #[test]
    fn a_convex_body_is_one_collision_primitive_per_hull() {
        let collision = Collision {
            hulls: vec![box_points(), box_points()],
            triangles: Vec::new(),
        };
        let glb = write(&quad(), &collision, "platform", GltfOptions::default()).unwrap();
        assert_eq!(glb.collision, Some((1, 2)));

        let doc = json_of(&glb);
        assert_eq!(doc["meshes"].as_array().unwrap().len(), 2);
        let parts = doc["meshes"][1]["primitives"].as_array().unwrap();
        assert_eq!(parts.len(), 2);
        // Collision primitives carry a position list and nothing else.
        assert!(parts[0]["material"].is_null());
        let attributes = parts[0]["attributes"].as_object().unwrap();
        assert_eq!(attributes.keys().collect::<Vec<_>>(), vec!["POSITION"]);
        // Only the node is in the scene, and it points at the visual mesh, so
        // drawing the glTF never draws the collision geometry.
        assert_eq!(doc["scenes"][0]["nodes"][0], 0);
        assert_eq!(doc["nodes"][0]["mesh"], 0);
        assert_eq!(doc["nodes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn a_trimesh_soup_is_one_indexed_collision_primitive() {
        let collision = Collision {
            hulls: Vec::new(),
            triangles: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 2.0),
            ],
        };
        let glb = write(&quad(), &collision, "world", GltfOptions::default()).unwrap();
        assert_eq!(glb.collision, Some((1, 1)));
        let doc = json_of(&glb);
        let soup = &doc["meshes"][1]["primitives"][0];
        let indices = &doc["accessors"][soup["indices"].as_u64().unwrap() as usize];
        assert_eq!(indices["count"], 3);
        assert_eq!(indices["type"], "SCALAR");
    }

    #[test]
    fn a_solid_drawn_only_in_clip_still_writes_its_collision() {
        let mut mesh = quad();
        mesh.surfaces[0].indices.clear();
        let collision = Collision {
            hulls: vec![box_points()],
            triangles: Vec::new(),
        };
        let glb = write(&mesh, &collision, "clip", GltfOptions::default()).unwrap();
        // No visual mesh, so collision takes glTF mesh 0.
        assert_eq!(glb.collision, Some((0, 1)));
        assert!(glb.textures.is_empty());
        let doc = json_of(&glb);
        assert_eq!(doc["meshes"].as_array().unwrap().len(), 1);
        // Nothing to draw, so no scene and no nodes.
        assert!(doc["scene"].is_null());
        assert!(doc["nodes"].is_null());
    }
    fn test_png(alpha: u8) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[12, 34, 56, alpha])
                .unwrap();
        }
        bytes
    }

    #[test]
    fn native_glb_embeds_regular_and_painted_albedo_with_distinct_samplers() {
        use crate::materials::{GeneratedMaterial, Material, MaterialLibrary};
        let root = std::env::temp_dir().join(format!("xport-gltf-albedo-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let png = test_png(255);
        // Detection follows the bytes, not the filename extension.
        std::fs::write(root.join("stone.dat"), &png).unwrap();
        std::fs::write(root.join("unsupported.webp"), b"RIFFxxxxWEBP").unwrap();
        let settings = crate::settings::Settings {
            project_root: Some(root.clone()),
            ..Default::default()
        };
        let mut library = MaterialLibrary::default();
        for name in ["stone", "stone_alias", "paint", "custom", "unsupported"] {
            library.materials.insert(
                name.into(),
                if name == "custom" {
                    Material::External {
                        res_path: "res://custom.tres".into(),
                    }
                } else {
                    Material::FromTextures {
                        albedo: if name == "unsupported" {
                            "res://unsupported.webp"
                        } else {
                            "res://stone.dat"
                        }
                        .into(),
                        maps: Vec::new(),
                    }
                },
            );
        }
        library.generated.insert(
            "paint".into(),
            GeneratedMaterial {
                pbr: None,
                surface: Some([0.2, 0.8, 2.5]),
                path: "res://paint.png".into(),
                width: 1,
                height: 1,
                png: png.clone(),
                alpha: false,
                repeat: false,
            },
        );
        let (images, warnings) = prepare_albedos(&library, &settings);
        assert_eq!(images["paint"].surface, Some([0.2, 0.8, 2.5]));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("PNG or JPEG"));
        assert_eq!(images.len(), 3);
        assert!(std::sync::Arc::ptr_eq(
            &images["stone"].bytes,
            &images["stone_alias"].bytes
        ));
        // The prepared library must remain sufficient after the files vanish.
        std::fs::remove_dir_all(&root).unwrap();
        let mut mesh = quad();
        for name in ["stone_alias", "paint", "custom", "unsupported"] {
            let mut surface = mesh.surfaces[0].clone();
            surface.texture = name.into();
            mesh.surfaces.push(surface);
        }
        let mut glb = plain(&mesh);
        embed_materials(&mut glb, &images, false);
        let doc = json_of(&glb);
        assert_eq!(doc["images"].as_array().unwrap().len(), 1); // identical bytes deduplicated
        assert_eq!(doc["images"][0]["mimeType"], "image/png");
        let paint = &doc["materials"][2];
        assert!(
            (paint["pbrMetallicRoughness"]["roughnessFactor"]
                .as_f64()
                .unwrap()
                - 0.2)
                .abs()
                < 1e-6
        );
        assert_eq!(
            paint["extensions"]["KHR_materials_emissive_strength"]["emissiveStrength"],
            2.5
        );
        assert_eq!(
            paint["emissiveTexture"]["index"],
            paint["pbrMetallicRoughness"]["baseColorTexture"]["index"]
        );
        assert_eq!(doc["textures"].as_array().unwrap().len(), 2); // repeat and clamp
        for (index, expected_wrap) in [(0, 10497), (1, 10497), (2, 33071)] {
            let material = &doc["materials"][index];
            let texture = material["pbrMetallicRoughness"]["baseColorTexture"]["index"]
                .as_u64()
                .unwrap() as usize;
            let sampler = doc["textures"][texture]["sampler"].as_u64().unwrap() as usize;
            assert_eq!(doc["samplers"][sampler]["wrapS"], expected_wrap);
            assert!(material.get("alphaMode").is_none()); // opaque RGBA must stay opaque
        }
        for index in [3, 4] {
            assert!(doc["materials"][index]
                .get("pbrMetallicRoughness")
                .is_none());
        }
        let view = doc["images"][0]["bufferView"].as_u64().unwrap() as usize;
        let offset = doc["bufferViews"][view]["byteOffset"].as_u64().unwrap() as usize;
        let json_size = u32::from_le_bytes(glb.bytes[12..16].try_into().unwrap()) as usize;
        assert_eq!(
            &glb.bytes[28 + json_size + offset..28 + json_size + offset + png.len()],
            png.as_slice()
        );
        assert_eq!(image_format(&test_png(128)), Some(("image/png", true)));
        assert_eq!(
            image_format(&[0xff, 0xd8, 0xff, 0xe0]),
            Some(("image/jpeg", false))
        );
        assert_eq!(image_format(b"not png or jpeg"), None);
    }

    #[test]
    fn tangents_follow_u_and_their_sign_puts_the_bitangent_image_up() {
        let mesh = quad();
        let s = &mesh.surfaces[0];
        let positions: Vec<[f32; 3]> = s
            .vertices
            .iter()
            .map(|v| {
                [
                    v.position.x as f32,
                    v.position.y as f32,
                    v.position.z as f32,
                ]
            })
            .collect();
        let normals: Vec<[f32; 3]> = s
            .vertices
            .iter()
            .map(|v| [v.normal.x as f32, v.normal.y as f32, v.normal.z as f32])
            .collect();
        let uvs: Vec<[f32; 2]> = s
            .vertices
            .iter()
            .map(|v| [v.uv.x as f32, v.uv.y as f32])
            .collect();
        // +U runs along +X and +V along +Z on this +Y quad, so image-up is -Z,
        // which is cross(normal, tangent) = cross(+Y, +X): w = +1.
        for t in tangents(&positions, &normals, &uvs, &s.indices, false) {
            assert!(
                (t[0] - 1.0).abs() < 1e-6 && t[1].abs() < 1e-6 && t[2].abs() < 1e-6,
                "{t:?}"
            );
            assert_eq!(t[3], 1.0);
        }
        // V written from the bottom shows the same picture on the same face, so the
        // frame must not change; reading the gradient the other way round would flip w.
        let flipped: Vec<[f32; 2]> = uvs.iter().map(|uv| [uv[0], 1.0 - uv[1]]).collect();
        assert!(tangents(&positions, &normals, &flipped, &s.indices, true)
            .iter()
            .all(|t| t[3] == 1.0));
        assert!(tangents(&positions, &normals, &flipped, &s.indices, false)
            .iter()
            .all(|t| t[3] == -1.0));
    }

    #[test]
    fn a_normal_map_is_embedded_with_tangents_and_texture_paint_keeps_repeating() {
        use crate::materials::{GeneratedMaterial, Material, MaterialLibrary};
        let png = test_png(255);
        let normal_png: std::sync::Arc<[u8]> = {
            let mut bytes = Vec::new();
            let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[128, 128, 255, 255])
                .unwrap();
            bytes.into()
        };
        let mut settings = crate::settings::Settings::default();
        settings
            .normal_maps
            .insert("tiled".into(), normal_png.clone());
        let mut library = MaterialLibrary::default();
        for (name, repeat) in [("tiled", true), ("paint", false)] {
            library.materials.insert(
                name.into(),
                Material::FromTextures {
                    albedo: format!("res://{name}.png"),
                    maps: Vec::new(),
                },
            );
            library.generated.insert(
                name.into(),
                GeneratedMaterial {
                    pbr: None,
                    path: format!("res://{name}.png"),
                    width: 1,
                    height: 1,
                    png: png.clone(),
                    alpha: false,
                    surface: Some([0.4, 0.6, 0.0]),
                    repeat,
                },
            );
        }
        let (images, _) = prepare_albedos(&library, &settings);
        let mut mesh = quad();
        mesh.surfaces[0].texture = "tiled".into();
        let mut paint = mesh.surfaces[0].clone();
        paint.texture = "paint".into();
        mesh.surfaces.push(paint);
        let mut glb = plain(&mesh);
        embed_materials(&mut glb, &images, false);
        let doc = json_of(&glb);
        let sampler = |material: &serde_json::Value, key: &str| {
            let texture = material[key]["index"].as_u64().unwrap() as usize;
            doc["textures"][texture]["sampler"].as_u64().unwrap()
        };
        let tiled = &doc["materials"][0];
        assert_eq!(
            sampler(&tiled["pbrMetallicRoughness"], "baseColorTexture"),
            0,
            "texture faces with a surface material tile"
        );
        assert_eq!(sampler(tiled, "normalTexture"), 0);
        assert_eq!(
            sampler(
                &doc["materials"][1]["pbrMetallicRoughness"],
                "baseColorTexture"
            ),
            1,
            "baked paint clamps"
        );
        assert!(doc["materials"][1].get("normalTexture").is_none());
        let primitives = doc["meshes"][0]["primitives"].as_array().unwrap();
        let tangent = primitives[0]["attributes"]["TANGENT"].as_u64().unwrap() as usize;
        assert!(primitives[1]["attributes"].get("TANGENT").is_none());
        let accessor = &doc["accessors"][tangent];
        assert_eq!(accessor["type"], "VEC4");
        assert_eq!(
            accessor["count"],
            doc["accessors"][primitives[0]["attributes"]["POSITION"].as_u64().unwrap() as usize]
                ["count"]
        );
        let json_size = u32::from_le_bytes(glb.bytes[12..16].try_into().unwrap()) as usize;
        let (values, _) = read_accessor(&doc, &glb.bytes[28 + json_size..], tangent).unwrap();
        assert_eq!(&values[..4], &[1.0, 0.0, 0.0, 1.0]);
        let image = doc["textures"][tiled["normalTexture"]["index"].as_u64().unwrap() as usize]
            ["source"]
            .as_u64()
            .unwrap() as usize;
        let view =
            &doc["bufferViews"][doc["images"][image]["bufferView"].as_u64().unwrap() as usize];
        let offset = view["byteOffset"].as_u64().unwrap() as usize;
        assert_eq!(
            &glb.bytes[28 + json_size + offset..28 + json_size + offset + normal_png.len()],
            &normal_png[..]
        );
    }
}
