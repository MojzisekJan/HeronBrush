//! Face selection by appearance, plane or edge connectivity.
use super::{Document, FaceRef, HashSet, Vec3d, node};
use geometry_math::bbox::Bbox3d;
use geometry_math::vec;
use map_model::polyhedron::FaceId;

const EDGE_TOLERANCE: f64 = 0.01;
const LINKED_ANGLE_DEGREES: f64 = 30.0;

/// How selected faces seed a broader, editable surface selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceSelection {
    /// Every editable face using any selected material.
    Material,
    /// Faces on the same oriented plane, including disconnected surfaces.
    Coplanar,
    /// Flood across shared edges while the material stays the same.
    LinkedMaterial,
    /// Flood across shared edges with at most 30 degrees between normals.
    LinkedAngle,
}

struct Surface<'a> {
    bounds: Bbox3d,
    reference: FaceRef,
    polygon: Vec<Vec3d>,
    normal: Vec3d,
    material: &'a str,
}

impl Document {
    /// Extends the selected surfaces, respecting hidden/locked layers and open groups.
    /// Angle traversal admits at most 30 degrees between adjacent faces.
    pub fn select_similar_faces(&mut self, mode: FaceSelection) -> usize {
        let seeds = self.selected_edit_faces();
        if seeds.is_empty() {
            return 0;
        }
        let open = self.open_group_contents();
        let mut surfaces = Vec::new();
        for (id, brush, layer) in node::brushes_by_layer(&self.tree.world) {
            if !self.is_editable_on(id, layer, &open) {
                continue;
            }
            for index in 0..brush.geometry().face_count() {
                let reference = FaceRef {
                    node: id,
                    face: index,
                };
                let Some(face) = self.face_at(reference) else {
                    continue;
                };
                let polygon = brush.geometry().face_positions(FaceId(index as u32));
                if polygon.is_empty() {
                    continue;
                }
                surfaces.push(Surface {
                    bounds: Bbox3d::merge_all(polygon.iter().copied())
                        .expect("nonempty face polygon"),
                    reference,
                    polygon,
                    normal: face.normal(),
                    material: face.material_name.as_ref(),
                });
            }
        }
        let mut chosen: HashSet<_> = surfaces
            .iter()
            .enumerate()
            .filter(|(_, s)| seeds.contains(&s.reference))
            .map(|(i, _)| i)
            .collect();
        let starts: Vec<_> = chosen.iter().copied().collect();
        match mode {
            FaceSelection::Material | FaceSelection::Coplanar => {
                for (i, candidate) in surfaces.iter().enumerate() {
                    if starts.iter().any(|&j| {
                        let seed = &surfaces[j];
                        match mode {
                            FaceSelection::Material => {
                                candidate.material.eq_ignore_ascii_case(seed.material)
                            }
                            _ => {
                                vec::length(candidate.normal - seed.normal) < 0.001
                                    && (vec::dot(
                                        seed.normal,
                                        candidate.polygon[0] - seed.polygon[0],
                                    ))
                                    .abs()
                                        < 0.01
                            }
                        }
                    }) {
                        chosen.insert(i);
                    }
                }
            }
            FaceSelection::LinkedMaterial | FaceSelection::LinkedAngle => {
                if surfaces.is_empty() {
                    return 0;
                }
                let index = SurfaceIndex::build((0..surfaces.len()).collect(), &surfaces);
                let mut queue = starts;
                while let Some(at) = queue.pop() {
                    let source = &surfaces[at];
                    let mut neighbors = Vec::new();
                    index.query(&source.bounds, &mut neighbors);
                    for i in neighbors {
                        if chosen.contains(&i) {
                            continue;
                        }
                        let candidate = &surfaces[i];
                        let compatible = match mode {
                            FaceSelection::LinkedMaterial => {
                                candidate.material.eq_ignore_ascii_case(source.material)
                            }
                            _ => {
                                vec::dot(candidate.normal, source.normal)
                                    >= (LINKED_ANGLE_DEGREES.to_radians()).cos() - 1e-6
                            }
                        };
                        if compatible && shares_edge(&source.polygon, &candidate.polygon) {
                            chosen.insert(i);
                            queue.push(i);
                        }
                    }
                }
            }
        }
        let faces: HashSet<_> = chosen.into_iter().map(|i| surfaces[i].reference).collect();
        let count = faces.len();
        if !faces.is_empty() {
            self.selection.clear();
            self.face_selection = faces;
            self.mark_selection_changed();
        }
        count
    }
}
fn shares_edge(a: &[Vec3d], b: &[Vec3d]) -> bool {
    (0..a.len()).any(|i| {
        (0..b.len()).any(|j| {
            super::selection::segments_overlap(
                (a[i], a[(i + 1) % a.len()]),
                (b[j], b[(j + 1) % b.len()]),
                EDGE_TOLERANCE,
            )
        })
    })
}

/// A local bounding-volume index keeps flood selection from comparing every pair
/// of faces in a large map. Exact overlapping-edge tests decide connectivity.
struct SurfaceIndex {
    bounds: Bbox3d,
    indices: Vec<usize>,
    children: Option<Box<[SurfaceIndex; 2]>>,
}
impl SurfaceIndex {
    fn build(mut indices: Vec<usize>, surfaces: &[Surface<'_>]) -> Self {
        let bounds = indices
            .iter()
            .map(|&i| surfaces[i].bounds)
            .reduce(|a, b| a.merged(&b))
            .expect("surface index is built from at least one face");
        if indices.len() <= 8 {
            return Self {
                bounds,
                indices,
                children: None,
            };
        }
        let size = bounds.size();
        let axis = (0..3)
            .max_by(|&a, &b| size[a].total_cmp(&size[b]))
            .expect("three coordinate axes");
        indices.sort_unstable_by(|&a, &b| {
            surfaces[a].bounds.center()[axis].total_cmp(&surfaces[b].bounds.center()[axis])
        });
        let right = indices.split_off(indices.len() / 2);
        Self {
            bounds,
            indices: Vec::new(),
            children: Some(Box::new([
                Self::build(indices, surfaces),
                Self::build(right, surfaces),
            ])),
        }
    }
    fn query(&self, bounds: &Bbox3d, out: &mut Vec<usize>) {
        if (0..3).any(|k| {
            self.bounds.min[k] > bounds.max[k] + EDGE_TOLERANCE
                || self.bounds.max[k] < bounds.min[k] - EDGE_TOLERANCE
        }) {
            return;
        }
        if let Some(children) = &self.children {
            for child in children.iter() {
                child.query(bounds, out);
            }
        } else {
            out.extend(&self.indices);
        }
    }
}
