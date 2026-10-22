//! Turns brushes into meshes.
//!
//! Each face starts as an oversized quad on its own plane and is clipped by
//! every other plane in the brush.

use crate::map::{Brush, Entity, Face, UvFormat};
use crate::math::{self, Aabb, Plane, Vec2, Vec3, VERTEX_EPSILON};
use crate::settings::{CollisionKind, OriginKind, Settings};
use indexmap::IndexMap;
use rayon::prelude::*;
use rustc_hash::FxHashMap as HashMap;

/// Texture dimensions in pixels, needed to normalise UVs.
#[derive(Debug, Default, Clone)]
pub struct TextureSizes {
    sizes: HashMap<String, Vec2>,
    fallback: Vec2,
}

impl TextureSizes {
    pub fn new(fallback: f64) -> Self {
        TextureSizes {
            sizes: HashMap::default(),
            fallback: Vec2::splat(fallback),
        }
    }

    pub fn insert(&mut self, texture: &str, size: Vec2) {
        self.sizes.insert(texture.to_ascii_lowercase(), size);
    }

    pub fn get(&self, texture: &str) -> Vec2 {
        self.sizes
            .get(&texture.to_ascii_lowercase())
            .copied()
            .unwrap_or(self.fallback)
    }
}

/// One mesh vertex, already in Godot's coordinate system and scale.
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

#[derive(Debug, Clone)]
pub struct Surface {
    pub texture: String,
    pub vertices: Vec<Vertex>,
    /// Triangle list into `vertices`.
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub surfaces: Vec<Surface>,
}

impl Mesh {
    pub fn is_empty(&self) -> bool {
        self.surfaces.iter().all(|s| s.indices.is_empty())
    }

    pub fn triangle_count(&self) -> usize {
        self.surfaces.iter().map(|s| s.indices.len() / 3).sum()
    }
}

/// Everything generated for one solid entity.
#[derive(Debug, Clone, Default)]
pub struct SolidGeometry {
    /// The entity's own origin, in map units and map axes.
    pub origin: Vec3,
    pub mesh: Mesh,
    /// One point cloud per brush, for `ConvexPolygonShape3D`.
    pub convex_hulls: Vec<Vec<Vec3>>,
    /// How many collision shapes joining neighbours saved.
    pub merged_hulls: usize,
    /// How many faces joining neighbours saved.
    pub merged_faces: usize,
    /// How many faces went because two brushes press together back to back.
    pub pressed_faces: usize,
    /// How many corners a join walked past and had to be given back.
    pub welded_corners: usize,
    /// What checking the finished geometry turned up, when asked for.
    pub problems: Vec<String>,
    /// The big flat faces to build an occluder from, when only some of them
    /// were asked for. Empty means use the whole mesh.
    pub occluder_faces: Vec<Vec<Vec3>>,
    /// Flat triangle list for `ConcavePolygonShape3D`.
    pub concave_triangles: Vec<Vec3>,
    /// Bounds in map units, before the origin is subtracted.
    pub bounds: Option<Aabb>,
    /// Brushes that never closed into a solid, by their index in the entity.
    pub unbounded_brushes: Vec<usize>,
}

/// A face polygon after clipping.
#[derive(Debug, Clone)]
struct FaceWinding {
    /// Set when an opposing face of another brush covers this one exactly, so
    /// it sits inside the solid and can never be seen.
    culled: bool,
    plane: Plane,
    vertices: Vec<Vec3>,
    normals: Vec<Vec3>,
    texture: String,
    uv: UvParams,
}

#[derive(Debug, Clone, Copy)]
struct UvParams {
    format: UvFormat,
    axes: [Vec3; 2],
    offset: Vec2,
    rotation: f64,
    scale: Vec2,
}

impl UvParams {
    fn from_face(face: &Face) -> Self {
        // A zero scale would divide the whole surface away.
        let scale = Vec2::new(
            if face.scale.x.abs() < 1e-9 {
                1.0
            } else {
                face.scale.x
            },
            if face.scale.y.abs() < 1e-9 {
                1.0
            } else {
                face.scale.y
            },
        );
        UvParams {
            format: face.format,
            axes: face.uv_axes,
            offset: face.offset,
            rotation: face.rotation,
            scale,
        }
    }
}

#[derive(Debug, Clone)]
struct BrushWinding {
    faces: Vec<FaceWinding>,
    /// Set when a face never got closed in, so the brush is not a solid.
    unbounded: bool,
    /// True when every face uses the origin texture. Such brushes only mark a
    /// pivot; they contribute no geometry and no collision.
    is_origin: bool,
}

/// UV coordinate for one vertex of a face, in texture space.
fn face_uv(vertex: Vec3, plane: &Plane, uv: &UvParams, texture_size: Vec2) -> Vec2 {
    let mut out = match uv.format {
        UvFormat::Valve220 => Vec2::new(
            uv.axes[0].dot(vertex) / uv.scale.x + uv.offset.x,
            uv.axes[1].dot(vertex) / uv.scale.y + uv.offset.y,
        ),
        UvFormat::Standard => {
            let (u_axis, v_axis) = standard_axes(plane.normal);
            let planar = Vec2::new(u_axis.dot(vertex), v_axis.dot(vertex));
            // The rotation field turns the texture, so the coordinates turn back.
            let r = -uv.rotation.to_radians();
            let (sin, cos) = r.sin_cos();
            let rotated = Vec2::new(
                planar.x * cos - planar.y * sin,
                planar.x * sin + planar.y * cos,
            );
            rotated / uv.scale + uv.offset
        }
    };
    out.x /= texture_size.x.max(1.0);
    out.y /= texture_size.y.max(1.0);
    out
}

/// The classic Quake texture axes, picked from the face's dominant axis.
fn standard_axes(normal: Vec3) -> (Vec3, Vec3) {
    let (nx, ny, nz) = (normal.x.abs(), normal.y.abs(), normal.z.abs());
    if nz >= nx && nz >= ny {
        // Floor and ceiling.
        (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0))
    } else if nx >= ny {
        // East and west walls.
        (Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, -1.0))
    } else {
        // North and south walls.
        (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0))
    }
}

/// True when the planes of a brush enclose a finite volume.
///
/// A brush missing a plane would otherwise clip into a huge stray polygon.
fn is_closed(planes: &[Plane]) -> bool {
    const AXES: [Vec3; 6] = [
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::Z,
        Vec3::NEG_Z,
    ];
    AXES.iter()
        .all(|axis| planes.iter().any(|p| p.normal.dot(*axis) > 1e-6))
}

/// True when a winding runs far past anything a real brush would cover, which
/// happens when the plane it sits on was never clipped back.
fn is_runaway(winding: &[Vec3]) -> bool {
    let limit = math::DEFAULT_HYPERPLANE_SIZE * 0.5;
    winding
        .iter()
        .any(|v| v.x.abs() > limit || v.y.abs() > limit || v.z.abs() > limit)
}

/// Clips every face of a brush against the others.
fn brush_windings(brush: &Brush, settings: &Settings, merge_distance: f64) -> BrushWinding {
    let planes: Vec<Plane> = brush.planes();
    let closed = is_closed(&planes);
    let mut faces = Vec::with_capacity(brush.faces.len());
    let mut is_origin = !brush.faces.is_empty();
    let mut scratch: Vec<Vec3> = Vec::new();

    for (i, face) in brush.faces.iter().enumerate() {
        if !settings.is_origin(&face.texture) {
            is_origin = false;
        }

        let mut winding = math::base_winding(&face.plane, math::DEFAULT_HYPERPLANE_SIZE);
        for (j, other) in planes.iter().enumerate() {
            if i == j || face.plane.is_equivalent(other, 1e-6) {
                continue;
            }
            math::clip_polygon_in_place(&mut winding, &mut scratch, other, 1e-5);
            if winding.is_empty() {
                break;
            }
        }

        if merge_distance > 0.0 {
            for v in &mut winding {
                *v = math::snap(*v, merge_distance);
            }
        }

        math::dedup_winding_in_place(&mut winding, VERTEX_EPSILON);
        if winding.len() < 3 || (!closed && is_runaway(&winding)) {
            // The face was clipped away entirely, which happens on brushes with
            // redundant planes. Keep the entry so face indices stay aligned.
            faces.push(FaceWinding {
                culled: false,
                plane: face.plane,
                vertices: Vec::new(),
                normals: Vec::new(),
                texture: face.texture.clone(),
                uv: UvParams::from_face(face),
            });
            continue;
        }
        math::sort_winding(&mut winding, face.plane.normal);

        let normals = vec![face.plane.normal; winding.len()];
        faces.push(FaceWinding {
            culled: false,
            plane: face.plane,
            vertices: winding,
            normals,
            texture: face.texture.clone(),
            uv: UvParams::from_face(face),
        });
    }

    BrushWinding {
        faces,
        is_origin,
        unbounded: !closed,
    }
}

/// Joins neighbouring faces sharing a plane, a texture and an alignment, where
/// the two of them make a convex polygon.
fn merge_coplanar_faces(brushes: &mut [BrushWinding], settings: &Settings) -> usize {
    // Faces that can be considered at all: drawn, flat, and with a shape.
    let mut live: Vec<(usize, usize)> = Vec::new();
    for (bi, brush) in brushes.iter().enumerate() {
        if brush.is_origin {
            continue;
        }
        for (fi, face) in brush.faces.iter().enumerate() {
            if face.culled || face.vertices.len() < 3 || settings.is_skip(&face.texture) {
                continue;
            }
            // Smoothing gives a face corners that disagree with its plane, and
            // the join would have to make up normals for the new corners.
            if face
                .normals
                .iter()
                .any(|n| n.dot(face.plane.normal) < 0.9999)
            {
                continue;
            }
            live.push((bi, fi));
        }
    }

    // Texture, plane and alignment: faces agreeing on all three can be joined.
    type SurfaceKey = (String, [i64; 4], [i64; 12]);
    let mut groups: HashMap<SurfaceKey, Vec<(usize, usize)>> = HashMap::default();
    for (bi, fi) in live.iter().copied() {
        let face = &brushes[bi].faces[fi];
        groups
            .entry((
                face.texture.clone(),
                plane_key(&face.plane),
                uv_key(&face.uv),
            ))
            .or_default()
            .push((bi, fi));
    }

    // Groups are worked out on every core: no two of them touch the same face,
    // so the only part that has to happen in order is writing the answers back.
    let groups: Vec<Vec<(usize, usize)>> = groups.into_values().collect();
    let done: Vec<(usize, Vec<Change>)> = if groups.len() >= 8 {
        groups
            .par_iter()
            .map(|g| join_one_group(g, brushes))
            .collect()
    } else {
        groups.iter().map(|g| join_one_group(g, brushes)).collect()
    };

    let mut joined = 0usize;
    for (count, changes) in done {
        joined += count;
        for change in changes {
            match change.corners {
                None => brushes[change.brush].faces[change.face].culled = true,
                Some(vertices) => {
                    let normal = brushes[change.brush].faces[change.face].plane.normal;
                    brushes[change.brush].faces[change.face].normals = vec![normal; vertices.len()];
                    brushes[change.brush].faces[change.face].vertices = vertices;
                }
            }
        }
    }
    joined
}

/// One face's new shape, or that it is gone.
struct Change {
    brush: usize,
    face: usize,
    corners: Option<Vec<Vec3>>,
}

/// Joins what can be joined among faces that share a plane, a texture and an
/// alignment, and reports what to write back.
fn join_one_group(group: &[(usize, usize)], brushes: &[BrushWinding]) -> (usize, Vec<Change>) {
    let mut joined = 0usize;
    if group.len() < 2 {
        return (0, Vec::new());
    }
    let plane = brushes[group[0].0].faces[group[0].1].plane;
    let (u, v) = plane_axes(plane.normal);
    let flat = |p: Vec3| Vec2::new(p.dot(u), p.dot(v));

    // Everything in the group laid out flat, kept here and updated in place
    // rather than read back out of the brushes every time.
    let mut shapes: Vec<Option<Vec<Vec2>>> = group
        .iter()
        .map(|(bi, fi)| {
            Some(
                brushes[*bi].faces[*fi]
                    .vertices
                    .iter()
                    .map(|p| flat(*p))
                    .collect(),
            )
        })
        .collect();
    // Sorted by where each face starts, so the sweep can stop early.
    let mut grew = vec![false; group.len()];
    let mut order: Vec<usize> = (0..group.len()).collect();
    order.sort_by(|a, b| {
        let (a_min, _) = flat_bounds(shapes[*a].as_ref().unwrap());
        let (b_min, _) = flat_bounds(shapes[*b].as_ref().unwrap());
        a_min
            .x
            .partial_cmp(&b_min.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut again = true;
    while again {
        again = false;
        for oa in 0..order.len() {
            let a = order[oa];
            if shapes[a].is_none() {
                continue;
            }
            for &b in order.iter().skip(oa + 1) {
                if shapes[b].is_none() {
                    continue;
                }
                let (a_min, a_max) = flat_bounds(shapes[a].as_ref().unwrap());
                let (b_min, b_max) = flat_bounds(shapes[b].as_ref().unwrap());
                // Past the end of this one, and so is everything after it.
                if b_min.x > a_max.x + 0.05 {
                    break;
                }
                if b_max.x < a_min.x - 0.05 || b_max.y < a_min.y - 0.05 || b_min.y > a_max.y + 0.05
                {
                    continue;
                }

                let one = shapes[a].as_ref().unwrap();
                let two = shapes[b].as_ref().unwrap();
                let mut both = one.clone();
                both.extend_from_slice(two);
                let mut hull = convex_hull(&both);
                if hull.len() < 3 {
                    continue;
                }
                // The hull walk always comes back counter-clockwise; a face pointing
                // the other way has to be turned back round or it is drawn from behind.
                if signed_area(&hull) * signed_area(one) < 0.0 {
                    hull.reverse();
                }
                // The hull is bigger than the two faces together exactly when their
                // union has a dent.
                let together = polygon_area(one) + polygon_area(two);
                if polygon_area(&hull) > together * 1.001 + 0.01 {
                    continue;
                }
                shapes[a] = Some(hull);
                shapes[b] = None;
                grew[a] = true;
                joined += 1;
                // No restart. This face carries on swallowing whatever else
                // it touches, and the sweep moves on when it runs out.
                again = true;
            }
        }
    }

    // Groups never share a face, so only writing has to take turns.
    let mut changes: Vec<Change> = Vec::new();
    for (slot, (bi, fi)) in group.iter().enumerate() {
        match &shapes[slot] {
            None => changes.push(Change {
                brush: *bi,
                face: *fi,
                corners: None,
            }),
            Some(points) => {
                if !grew[slot] {
                    continue;
                }
                let corner = brushes[*bi].faces[*fi].vertices[0];
                let base = corner - u * corner.dot(u) - v * corner.dot(v);
                let vertices: Vec<Vec3> = points.iter().map(|p| base + u * p.x + v * p.y).collect();
                changes.push(Change {
                    brush: *bi,
                    face: *fi,
                    corners: Some(vertices),
                });
            }
        }
    }
    (joined, changes)
}

fn flat_bounds(points: &[Vec2]) -> (Vec2, Vec2) {
    let mut low = Vec2::splat(f64::INFINITY);
    let mut high = Vec2::splat(f64::NEG_INFINITY);
    for p in points {
        low = low.min(*p);
        high = high.max(*p);
    }
    (low, high)
}

/// Whether the joined shape would leave some other face's corner sitting in the
/// middle of one of its edges. That is a T junction, and T junctions are where
/// hairline cracks come from.
/// Puts back the corners that a join walked past.
///
/// A vertex in the middle of somebody else's edge is a T junction, and the
/// renderer draws a hairline along it. Runs over a whole plane: a crack does
/// not care which texture is on either side.
fn weld_stray_corners(brushes: &mut [BrushWinding], settings: &Settings) -> usize {
    let mut live: Vec<(usize, usize)> = Vec::new();
    for (bi, brush) in brushes.iter().enumerate() {
        if brush.is_origin {
            continue;
        }
        for (fi, face) in brush.faces.iter().enumerate() {
            if face.culled || face.vertices.len() < 3 || settings.is_skip(&face.texture) {
                continue;
            }
            live.push((bi, fi));
        }
    }

    // By plane, which carries the direction the face looks. Two faces back to
    // back never crack against each other.
    let mut by_plane: HashMap<[i64; 4], Vec<usize>> = HashMap::default();
    for (place, (bi, fi)) in live.iter().enumerate() {
        by_plane
            .entry(plane_key(&brushes[*bi].faces[*fi].plane))
            .or_default()
            .push(place);
    }

    // Same again: planes do not share faces, so they can be welded all at once
    // and written back in order.
    let planes: Vec<Vec<usize>> = by_plane.into_values().collect();
    let done: Vec<Vec<(usize, usize, Vec<Vec3>)>> = if planes.len() >= 8 {
        planes
            .par_iter()
            .map(|group| weld_one_plane(group, &live, brushes))
            .collect()
    } else {
        planes
            .iter()
            .map(|group| weld_one_plane(group, &live, brushes))
            .collect()
    };

    let mut welded = 0usize;
    for changes in done {
        for (bi, fi, corners) in changes {
            welded += corners.len() - brushes[bi].faces[fi].vertices.len();
            let normal = brushes[bi].faces[fi].plane.normal;
            brushes[bi].faces[fi].normals = vec![normal; corners.len()];
            brushes[bi].faces[fi].vertices = corners;
        }
    }
    welded
}

/// Every face on one plane that needs corners put back, with its new corners.
fn weld_one_plane(
    group: &[usize],
    live: &[(usize, usize)],
    brushes: &[BrushWinding],
) -> Vec<(usize, usize, Vec<Vec3>)> {
    let mut out = Vec::new();
    if group.len() < 2 {
        return out;
    }
    // Sorted along x, so an edge can ask for the corners inside its own span
    // instead of walking the whole plane.
    let mut corners: Vec<(f64, Vec3, usize)> = Vec::new();
    for slot in group.iter() {
        let (bi, fi) = live[*slot];
        for p in &brushes[bi].faces[fi].vertices {
            corners.push((p.x, *p, *slot));
        }
    }
    corners.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    for slot in group.iter() {
        let (bi, fi) = live[*slot];
        let mine = &brushes[bi].faces[fi].vertices;
        let mut grown: Vec<Vec3> = Vec::with_capacity(mine.len());

        for corner in 0..mine.len() {
            let from = mine[corner];
            let to = mine[(corner + 1) % mine.len()];
            grown.push(from);
            let low = from.min(to) - Vec3::splat(VERTEX_EPSILON);
            let high = from.max(to) + Vec3::splat(VERTEX_EPSILON);

            let first = corners.partition_point(|(x, _, _)| *x < low.x);
            let mut sitting: Vec<Vec3> = Vec::new();
            for (x, p, owner) in corners[first..].iter() {
                if *x > high.x {
                    break;
                }
                if owner == slot || p.y < low.y || p.y > high.y || p.z < low.z || p.z > high.z {
                    continue;
                }
                if sits_along(*p, from, to)
                    && !sitting.iter().any(|q| q.distance_squared(*p) < 1e-8)
                {
                    sitting.push(*p);
                }
            }
            sitting.sort_by(|a, b| {
                from.distance_squared(*a)
                    .partial_cmp(&from.distance_squared(*b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            grown.extend(sitting);
        }

        if grown.len() != mine.len() {
            out.push((bi, fi, grown));
        }
    }
    out
}

/// Bends a patch into triangles and adds them to a surface.
///
/// Every three by three corner of the control grid is one biquadratic bezier
/// surface, evaluated on a grid of its own. The corners of neighbouring pieces
/// land on the same points, so the seams close by themselves.
fn add_patch(
    patch: &crate::map::Patch,
    surface: &mut Surface,
    settings: &Settings,
    to_local: &dyn Fn(Vec3) -> Vec3,
) {
    let (steps_u, steps_v) = patch
        .subdivisions
        .unwrap_or((settings.patch_subdivisions, settings.patch_subdivisions));
    let steps_u = steps_u.clamp(1, 32);
    let steps_v = steps_v.clamp(1, 32);
    let at = |x: usize, y: usize| x * patch.height + y;

    let mut piece_u = 0;
    while piece_u + 2 < patch.width {
        let mut piece_v = 0;
        while piece_v + 2 < patch.height {
            let base = surface.vertices.len() as u32;
            for su in 0..=steps_u {
                let u = su as f64 / steps_u as f64;
                for sv in 0..=steps_v {
                    let v = sv as f64 / steps_v as f64;
                    let mut point = Vec3::ZERO;
                    let mut uv = Vec2::ZERO;
                    let mut along_u = Vec3::ZERO;
                    let mut along_v = Vec3::ZERO;
                    for a in 0..3 {
                        for b in 0..3 {
                            let index = at(piece_u + a, piece_v + b);
                            let (wu, du) = (bernstein(a, u), bernstein_slope(a, u));
                            let (wv, dv) = (bernstein(b, v), bernstein_slope(b, v));
                            point += patch.points[index] * (wu * wv);
                            uv += patch.uvs[index] * (wu * wv);
                            along_u += patch.points[index] * (du * wv);
                            along_v += patch.points[index] * (wu * dv);
                        }
                    }
                    // At a corner where the patch pinches to a point one of the
                    // two directions vanishes; the other piece of the seam has
                    // a normal there, and smoothing is what shares it.
                    let normal = along_u.cross(along_v);
                    let normal = if normal.length_squared() > 1e-12 {
                        normal.normalize()
                    } else {
                        Vec3::Z
                    };
                    surface.vertices.push(Vertex {
                        position: to_local(point),
                        // Against the winding, the same way round as a brush
                        // face, or the lighting comes out inside out.
                        normal: math::to_godot(-normal),
                        uv,
                    });
                }
            }
            let row = (steps_v + 1) as u32;
            for su in 0..steps_u as u32 {
                for sv in 0..steps_v as u32 {
                    let a = base + su * row + sv;
                    let b = a + row;
                    // Wound the same way as brush faces, which the axis swizzle
                    // turns round on the way into Godot.
                    surface
                        .indices
                        .extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
                }
            }
            piece_v += 2;
        }
        piece_u += 2;
    }
}

/// The three weights of a quadratic bezier, and how fast each one changes.
fn bernstein(i: usize, t: f64) -> f64 {
    match i {
        0 => (1.0 - t) * (1.0 - t),
        1 => 2.0 * t * (1.0 - t),
        _ => t * t,
    }
}

fn bernstein_slope(i: usize, t: f64) -> f64 {
    match i {
        0 => 2.0 * t - 2.0,
        1 => 2.0 - 4.0 * t,
        _ => 2.0 * t,
    }
}

/// Triangles for one convex face, as index triples into its own corners.
///
/// Not a fan: welding leaves vertices in the middle of straight edges, and a
/// fan spans them, which puts back the long edge and makes an empty triangle.
fn triangles_of(points: &[Vec3]) -> Vec<[usize; 3]> {
    let mut left: Vec<usize> = (0..points.len()).collect();
    let mut out: Vec<[usize; 3]> = Vec::with_capacity(points.len().saturating_sub(2));
    while left.len() > 3 {
        let mut turning = None;
        for i in 0..left.len() {
            let before = points[left[(i + left.len() - 1) % left.len()]];
            let here = points[left[i]];
            let after = points[left[(i + 1) % left.len()]];
            if (here - before).cross(after - here).length_squared() > 1e-12 {
                turning = Some(i);
                break;
            }
        }
        // Every corner in a line: nothing left with any area on it.
        let Some(i) = turning else { return out };
        out.push([
            left[(i + left.len() - 1) % left.len()],
            left[i],
            left[(i + 1) % left.len()],
        ]);
        left.remove(i);
    }
    if left.len() == 3 {
        let (a, b, c) = (points[left[0]], points[left[1]], points[left[2]]);
        if (b - a).cross(c - a).length_squared() > 1e-12 {
            out.push([left[0], left[1], left[2]]);
        }
    }
    out
}

/// Everything wrong with a finished mesh that a person would only find by
/// walking up to it: faces drawn from behind, triangles with no area, and
/// corners left in the middle of somebody else's edge.
///
/// These are the shapes the bugs took, so `heron_brush check` looks for them on
/// whatever a mapper draws rather than only on hand-written test brushes.
fn look_for_trouble(mesh: &Mesh) -> Vec<String> {
    let mut found = Vec::new();
    for surface in &mesh.surfaces {
        let (mut with, mut against, mut flat) = (0usize, 0usize, 0usize);
        for tri in surface.indices.chunks(3) {
            let a = surface.vertices[tri[0] as usize];
            let b = surface.vertices[tri[1] as usize];
            let c = surface.vertices[tri[2] as usize];
            let wound = (b.position - a.position).cross(c.position - a.position);
            if wound.length() < 1e-9 {
                flat += 1;
                continue;
            }
            if wound.normalize().dot(a.normal) < 0.0 {
                against += 1;
            } else {
                with += 1;
            }
        }
        if with > 0 && against > 0 {
            found.push(format!(
                "`{}`: {} triangles face the other way from the rest, so they are drawn from behind",
                surface.texture,
                with.min(against)
            ));
        }
        if flat > 0 {
            found.push(format!(
                "`{}`: {flat} triangles with no area",
                surface.texture
            ));
        }
        let (stranded, first) = stranded_corners(surface);
        if stranded > 0 {
            found.push(format!(
                "`{}`: {stranded} corners sit in the middle of an edge, which draws as a bright hairline, one of them around ({:.0} {:.0} {:.0}) in map units",
                surface.texture, first.z * 32.0, first.x * 32.0, first.y * 32.0
            ));
        }
    }
    found
}

/// Corners sitting inside another triangle's edge. Sorted along x and swept, so
/// a big map does not turn this into every corner against every edge.
fn stranded_corners(surface: &Surface) -> (usize, Vec3) {
    let mut along_x: Vec<usize> = (0..surface.vertices.len()).collect();
    along_x.sort_by(|a, b| {
        surface.vertices[*a]
            .position
            .x
            .partial_cmp(&surface.vertices[*b].position.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let sorted: Vec<f64> = along_x
        .iter()
        .map(|i| surface.vertices[*i].position.x)
        .collect();

    // An edge two triangles share is inside a face, and a surface cannot crack
    // along it however many corners sit there. Only the edges on the outside of
    // a face are worth asking about.
    let mut times_used: HashMap<[[i64; 3]; 3], usize> = HashMap::default();
    for tri in surface.indices.chunks(3) {
        for edge in 0..3 {
            let key = edge_key(
                surface.vertices[tri[edge] as usize].position,
                surface.vertices[tri[(edge + 1) % 3] as usize].position,
                surface.vertices[tri[edge] as usize].normal,
            );
            *times_used.entry(key).or_insert(0) += 1;
        }
    }

    let mut found = 0;
    let mut where_first = Vec3::ZERO;
    for tri in surface.indices.chunks(3) {
        for edge in 0..3 {
            let from = surface.vertices[tri[edge] as usize];
            let to = surface.vertices[tri[(edge + 1) % 3] as usize];
            if times_used
                .get(&edge_key(from.position, to.position, from.normal))
                .copied()
                .unwrap_or(0)
                > 1
            {
                continue;
            }
            let low = from.position.x.min(to.position.x) - VERTEX_EPSILON;
            let high = from.position.x.max(to.position.x) + VERTEX_EPSILON;
            let first = sorted.partition_point(|x| *x < low);
            for (place, x) in sorted.iter().enumerate().skip(first) {
                if *x > high {
                    break;
                }
                let i = along_x[place];
                if tri.contains(&(i as u32)) {
                    continue;
                }
                let other = surface.vertices[i];
                // Only faces looking the same way can crack against each other.
                if other.normal.dot(from.normal) < 0.999 {
                    continue;
                }
                if sits_along(other.position, from.position, to.position) {
                    if found == 0 {
                        where_first = other.position;
                    }
                    found += 1;
                }
            }
        }
    }
    (found, where_first)
}

/// A key that is the same for an edge whichever way round it is walked, and
/// different for the two faces of a corner. The edge along the top of a wall
/// belongs to the wall and to the ceiling, and those two are not one surface.
fn edge_key(a: Vec3, b: Vec3, facing: Vec3) -> [[i64; 3]; 3] {
    let (one, two) = (quantize(a), quantize(b));
    let n = quantize(facing * 100.0);
    if one <= two {
        [one, two, n]
    } else {
        [two, one, n]
    }
}

/// Whether the point is on the segment, strictly between its ends.
fn sits_along(p: Vec3, from: Vec3, to: Vec3) -> bool {
    let along = to - from;
    let length_squared = along.length_squared();
    if length_squared < 1e-12 {
        return false;
    }
    let t = (p - from).dot(along) / length_squared;
    if !(0.0001..0.9999).contains(&t) {
        return false;
    }
    (from + along * t).distance_squared(p) < VERTEX_EPSILON * VERTEX_EPSILON
}

/// Corners of the smallest convex shape holding all the points, by Andrew's
/// monotone chain.
fn convex_hull(points: &[Vec2]) -> Vec<Vec2> {
    let mut sorted: Vec<Vec2> = Vec::with_capacity(points.len());
    for p in points {
        if !sorted.iter().any(|q: &Vec2| q.distance_squared(*p) < 1e-8) {
            sorted.push(*p);
        }
    }
    if sorted.len() < 3 {
        return Vec::new();
    }
    sorted.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });

    let turns_back = |hull: &[Vec2], p: Vec2| -> bool {
        let last = hull[hull.len() - 1];
        let before = hull[hull.len() - 2];
        (last - before).perp_dot(p - last) <= 1e-9
    };

    let mut hull: Vec<Vec2> = Vec::with_capacity(sorted.len() * 2);
    for p in sorted.iter() {
        while hull.len() >= 2 && turns_back(&hull, *p) {
            hull.pop();
        }
        hull.push(*p);
    }
    // The top half may not eat into the bottom one, so it stops one short of
    // where the bottom half ended.
    let floor = hull.len() + 1;
    for p in sorted.iter().rev().skip(1) {
        while hull.len() >= floor && turns_back(&hull, *p) {
            hull.pop();
        }
        hull.push(*p);
    }
    // The walk ends where it started.
    hull.pop();
    hull
}

fn polygon_area(points: &[Vec2]) -> f64 {
    signed_area(points).abs()
}

/// Area with a sign. Which way the points run is which side the renderer
/// draws, so a join has to keep it.
fn signed_area(points: &[Vec2]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut twice = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        twice += a.perp_dot(b);
    }
    twice * 0.5
}

/// Two directions in a plane, for working in it as if it were flat.
fn plane_axes(normal: Vec3) -> (Vec3, Vec3) {
    let away = if normal.x.abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let u = normal.cross(away).normalize();
    (u, normal.cross(u).normalize())
}

fn plane_key(plane: &Plane) -> [i64; 4] {
    [
        (plane.normal.x * 4096.0).round() as i64,
        (plane.normal.y * 4096.0).round() as i64,
        (plane.normal.z * 4096.0).round() as i64,
        (plane.d * 16.0).round() as i64,
    ]
}

/// Every number `face_uv` reads. Not the rotation field: Valve 220 writes the
/// turn into the axes, but editors still fill the field in, differently
/// on either side of a cut, and keying on it stopped anything joining.
fn uv_key(uv: &UvParams) -> [i64; 12] {
    let fine = |f: f64| (f * 4096.0).round() as i64;
    let rotation_matters = matches!(uv.format, UvFormat::Standard);
    [
        fine(uv.axes[0].x),
        fine(uv.axes[0].y),
        fine(uv.axes[0].z),
        fine(uv.axes[1].x),
        fine(uv.axes[1].y),
        fine(uv.axes[1].z),
        (uv.offset.x * 64.0).round() as i64,
        (uv.offset.y * 64.0).round() as i64,
        fine(uv.scale.x),
        fine(uv.scale.y),
        if rotation_matters {
            (uv.rotation * 64.0).round() as i64
        } else {
            0
        },
        rotation_matters as i64,
    ]
}

/// Marks faces that two brushes press together, back to back.
///
/// Only exact matches: a partly buried face would have to be cut up, which
/// leaves T junctions.
fn cull_interior_faces(brushes: &mut [BrushWinding]) -> usize {
    // Faces are keyed by their vertex set, which is identical for the two
    // halves of a shared boundary regardless of where each winding starts.
    let mut by_shape: HashMap<Vec<[i64; 3]>, Vec<(usize, usize)>> = HashMap::default();
    for (bi, brush) in brushes.iter().enumerate() {
        if brush.is_origin {
            continue;
        }
        for (fi, face) in brush.faces.iter().enumerate() {
            if face.vertices.len() < 3 {
                continue;
            }
            let mut key: Vec<[i64; 3]> = face.vertices.iter().map(|v| quantize(*v)).collect();
            key.sort_unstable();
            by_shape.entry(key).or_default().push((bi, fi));
        }
    }

    let mut culled = 0usize;
    for group in by_shape.values() {
        if group.len() < 2 {
            continue;
        }
        let mut taken = vec![false; group.len()];
        for a in 0..group.len() {
            if taken[a] {
                continue;
            }
            let (ba, fa) = group[a];
            for b in (a + 1)..group.len() {
                if taken[b] {
                    continue;
                }
                let (bb, fb) = group[b];
                if ba == bb {
                    continue;
                }
                let na = brushes[ba].faces[fa].plane.normal;
                let nb = brushes[bb].faces[fb].plane.normal;
                if na.dot(nb) > -0.999 {
                    continue;
                }
                taken[a] = true;
                taken[b] = true;
                brushes[ba].faces[fa].culled = true;
                brushes[bb].faces[fb].culled = true;
                culled += 2;
                break;
            }
        }
    }
    culled
}

fn quantize(v: Vec3) -> [i64; 3] {
    let step = VERTEX_EPSILON;
    [
        (v.x / step).round() as i64,
        (v.y / step).round() as i64,
        (v.z / step).round() as i64,
    ]
}

/// Averages vertex normals across faces that meet at a shallow enough angle.
fn smooth_normals(brushes: &mut [BrushWinding], angle_degrees: f64) {
    let limit = angle_degrees.to_radians();
    let mut groups: HashMap<[i64; 3], Vec<(usize, usize, usize)>> = HashMap::default();

    for (bi, brush) in brushes.iter().enumerate() {
        for (fi, face) in brush.faces.iter().enumerate() {
            for (vi, v) in face.vertices.iter().enumerate() {
                groups.entry(quantize(*v)).or_default().push((bi, fi, vi));
            }
        }
    }

    for members in groups.values() {
        if members.len() < 2 {
            continue;
        }
        let originals: Vec<Vec3> = members
            .iter()
            .map(|(bi, fi, vi)| brushes[*bi].faces[*fi].normals[*vi])
            .collect();

        let mut smoothed = Vec::with_capacity(members.len());
        for (i, this) in originals.iter().enumerate() {
            let mut sum = *this;
            for (j, other) in originals.iter().enumerate() {
                if i == j {
                    continue;
                }
                if this.dot(*other).clamp(-1.0, 1.0).acos() <= limit {
                    sum += *other;
                }
            }
            smoothed.push(sum.normalize_or_zero());
        }

        for (slot, (bi, fi, vi)) in members.iter().enumerate() {
            let n = smoothed[slot];
            if n != Vec3::ZERO {
                brushes[*bi].faces[*fi].normals[*vi] = n;
            }
        }
    }
}

fn bounds_of(brushes: &[BrushWinding], only_origin_brushes: bool) -> Option<Aabb> {
    Aabb::from_points(
        brushes
            .iter()
            .filter(|b| b.is_origin == only_origin_brushes)
            .flat_map(|b| b.faces.iter())
            .flat_map(|f| f.vertices.iter().copied()),
    )
}

fn resolve_origin(
    entity: &Entity,
    brushes: &[BrushWinding],
    kind: OriginKind,
    settings: &Settings,
) -> Vec3 {
    let all = bounds_of(brushes, false).or_else(|| bounds_of(brushes, true));
    let center = all.map(|b| b.center()).unwrap_or(Vec3::ZERO);

    match kind {
        OriginKind::BoundsCenter => center,
        OriginKind::BoundsMins => all.map(|b| b.min).unwrap_or(Vec3::ZERO),
        OriginKind::BoundsMaxs => all.map(|b| b.max).unwrap_or(Vec3::ZERO),
        OriginKind::Averaged => {
            let mut sum = Vec3::ZERO;
            let mut count = 0usize;
            for b in brushes {
                for f in &b.faces {
                    for v in &f.vertices {
                        sum += *v;
                        count += 1;
                    }
                }
            }
            if count == 0 {
                Vec3::ZERO
            } else {
                sum / count as f64
            }
        }
        OriginKind::Absolute => entity.get_vec3("origin").unwrap_or(center),
        OriginKind::Relative => center + entity.get_vec3("origin").unwrap_or(Vec3::ZERO),
        OriginKind::Brush => match bounds_of(brushes, true) {
            Some(b) => b.center(),
            None => {
                // No origin brush. An explicit `origin` property is the next
                // best pivot, then the bounding box centre.
                let _ = settings;
                entity.get_vec3("origin").unwrap_or(center)
            }
        },
    }
}

/// Options that vary per entity, taken from its class definition.
#[derive(Debug, Clone, Copy)]
pub struct SolidOptions {
    pub origin_kind: OriginKind,
    pub collision: CollisionKind,
    pub build_visuals: bool,
    /// Worldspawn keeps its geometry in world space.
    pub force_zero_origin: bool,
    /// Build the occluder from faces at least this big, in square map units,
    /// rather than from the whole mesh. Zero means the whole mesh.
    pub occluder_min_area: f64,
}

impl Default for SolidOptions {
    fn default() -> Self {
        SolidOptions {
            origin_kind: OriginKind::Brush,
            collision: CollisionKind::Convex,
            build_visuals: true,
            force_zero_origin: false,
            occluder_min_area: 0.0,
        }
    }
}

/// Area of a flat polygon, by fanning it into triangles.
fn face_area(points: &[Vec3]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut total = 0.0;
    for i in 1..points.len() - 1 {
        total += (points[i] - points[0])
            .cross(points[i + 1] - points[0])
            .length()
            * 0.5;
    }
    total
}

/// The corners of one brush, in the solid's own space, without repeats.
fn local_points(brush: &BrushWinding, to_local: &impl Fn(Vec3) -> Vec3) -> Vec<Vec3> {
    let mut points: Vec<Vec3> = Vec::new();
    for face in &brush.faces {
        for v in &face.vertices {
            let p = to_local(*v);
            if !points.iter().any(|q| q.distance_squared(p) < 1e-12) {
                points.push(p);
            }
        }
    }
    points
}

/// Builds mesh and collision data for one solid entity.
pub fn build_solid(
    entity: &Entity,
    settings: &Settings,
    options: SolidOptions,
    textures: &TextureSizes,
) -> SolidGeometry {
    let merge_distance = entity
        .get_f64(&settings.vertex_merge_property)
        .unwrap_or(0.0);
    // One brush's windings have nothing to do with any other's. Small entities
    // stay on one core, where a thread pool costs more than it saves.
    let mut brushes: Vec<BrushWinding> = if entity.brushes.len() >= 64 {
        entity
            .brushes
            .par_iter()
            .map(|b| brush_windings(b, settings, merge_distance))
            .collect()
    } else {
        entity
            .brushes
            .iter()
            .map(|b| brush_windings(b, settings, merge_distance))
            .collect()
    };

    let smoothing_on = entity
        .get(&settings.smoothing_property)
        .map(|v| v != "0" && !v.is_empty() && !v.eq_ignore_ascii_case("false"))
        .unwrap_or(false);
    if smoothing_on {
        let angle = entity
            .get_f64(&settings.smoothing_angle_property)
            .unwrap_or(89.0);
        smooth_normals(&mut brushes, angle);
    }

    // On unless the entity says otherwise. Two brushes pressed back to back in
    // one entity cannot come apart, so the pair of faces between them is never
    // seen. A mitred corner is the everyday case.
    let cull_interior = entity
        .get(&settings.cull_interior_property)
        .map(|v| v != "0" && !v.is_empty() && !v.eq_ignore_ascii_case("false"))
        .unwrap_or(settings.cull_hidden);
    let pressed_faces = if cull_interior {
        cull_interior_faces(&mut brushes)
    } else {
        0
    };

    let mut merged_faces = 0usize;
    let mut welded_corners = 0usize;
    if settings.merge_faces {
        merged_faces = merge_coplanar_faces(&mut brushes, settings);
        // A join can grow a face past a corner that cannot join it, and that corner
        // has to go back into the edge or it is a hairline crack.
        welded_corners = weld_stray_corners(&mut brushes, settings);
    }

    let mut bounds = bounds_of(&brushes, false).or_else(|| bounds_of(&brushes, true));
    if !entity.patches.is_empty() {
        let corners = entity.patches.iter().flat_map(|p| p.points.iter().copied());
        if let Some(around) = Aabb::from_points(corners) {
            bounds = Some(match bounds {
                Some(b) => Aabb {
                    min: b.min.min(around.min),
                    max: b.max.max(around.max),
                },
                None => around,
            });
        }
    }
    if let Some(around) = Aabb::from_points(
        entity
            .external_surfaces
            .iter()
            .flat_map(|s| s.vertices.iter().map(|v| v.position)),
    ) {
        bounds = Some(match bounds {
            Some(b) => Aabb {
                min: b.min.min(around.min),
                max: b.max.max(around.max),
            },
            None => around,
        });
    }
    let origin = if options.force_zero_origin {
        Vec3::ZERO
    } else {
        resolve_origin(entity, &brushes, options.origin_kind, settings)
    };

    let scale = settings.scale_factor;
    let to_local = |v: Vec3| math::to_godot(v - origin) * scale;

    let mut geometry = SolidGeometry {
        merged_faces,
        pressed_faces,
        welded_corners,
        origin,
        bounds,
        unbounded_brushes: brushes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.unbounded)
            .map(|(i, _)| i)
            .collect(),
        ..Default::default()
    };

    if options.build_visuals {
        // One surface per texture, in first-seen order so rebuilds stay stable.
        let mut surfaces: IndexMap<String, Surface> = IndexMap::new();
        for incoming in &entity.external_surfaces {
            let mut surface = incoming.clone();
            for vertex in &mut surface.vertices {
                vertex.position = to_local(vertex.position);
                vertex.normal = math::to_godot(vertex.normal);
            }
            let target = surfaces
                .entry(surface.texture.clone())
                .or_insert_with(|| Surface {
                    texture: surface.texture.clone(),
                    vertices: Vec::new(),
                    indices: Vec::new(),
                });
            let offset = target.vertices.len() as u32;
            target.vertices.extend(surface.vertices);
            target
                .indices
                .extend(surface.indices.into_iter().map(|i| i + offset));
        }

        for brush in &brushes {
            if brush.is_origin {
                continue;
            }
            for face in &brush.faces {
                if face.culled
                    || face.vertices.len() < 3
                    || settings.is_skip(&face.texture)
                    || settings.is_origin(&face.texture)
                    || settings.is_clip(&face.texture)
                {
                    continue;
                }
                let size = textures.get(&face.texture);
                let surface = surfaces
                    .entry(face.texture.clone())
                    .or_insert_with(|| Surface {
                        texture: face.texture.clone(),
                        vertices: Vec::new(),
                        indices: Vec::new(),
                    });

                let base = surface.vertices.len() as u32;
                for (i, v) in face.vertices.iter().enumerate() {
                    surface.vertices.push(Vertex {
                        position: to_local(*v),
                        normal: math::to_godot(face.normals[i]),
                        uv: face_uv(*v, &face.plane, &face.uv, size),
                    });
                }
                for [a, b, c] in triangles_of(&face.vertices) {
                    surface.indices.extend_from_slice(&[
                        base + a as u32,
                        base + b as u32,
                        base + c as u32,
                    ]);
                }
            }
        }
        for patch in &entity.patches {
            if settings.is_skip(&patch.texture) || settings.is_origin(&patch.texture) {
                continue;
            }
            let surface = surfaces
                .entry(patch.texture.clone())
                .or_insert_with(|| Surface {
                    texture: patch.texture.clone(),
                    vertices: Vec::new(),
                    indices: Vec::new(),
                });
            add_patch(patch, surface, settings, &to_local);
        }
        geometry.mesh.surfaces = surfaces.into_values().collect();
    }

    // An occluder is drawn on the processor every frame, so a hundred small ones
    // cost more than they save.
    if options.occluder_min_area > 0.0 {
        for brush in &brushes {
            if brush.is_origin {
                continue;
            }
            for face in &brush.faces {
                if face.culled
                    || face.vertices.len() < 3
                    || settings.is_skip(&face.texture)
                    || settings.is_clip(&face.texture)
                {
                    continue;
                }
                if face_area(&face.vertices) < options.occluder_min_area {
                    continue;
                }
                geometry
                    .occluder_faces
                    .push(face.vertices.iter().map(|v| to_local(*v)).collect());
            }
        }
    }

    match options.collision {
        CollisionKind::None => {}
        CollisionKind::Convex => {
            // Neighbouring brushes are joined first, where the two together are still
            // convex: a wall drawn as eight blocks becomes one box for the physics.
            let mut plane_sets: Vec<Vec<Plane>> = Vec::new();
            for brush in &brushes {
                if brush.is_origin {
                    continue;
                }
                if brush.unbounded {
                    let points = local_points(brush, &to_local);
                    if points.len() >= 4 {
                        geometry.convex_hulls.push(points);
                    }
                    continue;
                }
                plane_sets.push(brush.faces.iter().map(|f| f.plane).collect());
            }
            let count = plane_sets.len();
            let joined = if settings.merge_collision && count > 1 {
                crate::merge::convex_groups(plane_sets, count)
            } else {
                plane_sets
            };
            geometry.merged_hulls = count.saturating_sub(joined.len());
            for planes in joined {
                let mut points: Vec<Vec3> = Vec::new();
                for corner in crate::overlap::corners_of(&planes) {
                    let p = to_local(corner);
                    if !points.iter().any(|q| q.distance_squared(p) < 1e-12) {
                        points.push(p);
                    }
                }
                if points.len() >= 4 {
                    geometry.convex_hulls.push(points);
                }
            }
        }
        CollisionKind::Concave => {
            for surface in &entity.external_surfaces {
                for index in &surface.indices {
                    if let Some(vertex) = surface.vertices.get(*index as usize) {
                        geometry.concave_triangles.push(to_local(vertex.position));
                    }
                }
            }
            // A patch is a surface, not a solid, so it can only ever be part of
            // a concave shape. An entity that wants convex collision gets none
            // from its patches.
            for patch in &entity.patches {
                if settings.is_skip(&patch.texture) {
                    continue;
                }
                let mut scratch = Surface {
                    texture: patch.texture.clone(),
                    vertices: Vec::new(),
                    indices: Vec::new(),
                };
                add_patch(patch, &mut scratch, settings, &to_local);
                for index in &scratch.indices {
                    geometry
                        .concave_triangles
                        .push(scratch.vertices[*index as usize].position);
                }
            }
            for brush in &brushes {
                if brush.is_origin {
                    continue;
                }
                for face in &brush.faces {
                    // Culled faces are inside the solid or already covered by the face that
                    // ate them. Left in, four blocks in a row cost 48 triangles instead of 12.
                    if face.culled
                        || face.vertices.len() < 3
                        || (settings.is_skip(&face.texture)
                            && face.texture != crate::settings::EXPORT_HIDDEN_TEXTURE)
                    {
                        continue;
                    }
                    for [a, b, c] in triangles_of(&face.vertices) {
                        geometry.concave_triangles.push(to_local(face.vertices[a]));
                        geometry.concave_triangles.push(to_local(face.vertices[b]));
                        geometry.concave_triangles.push(to_local(face.vertices[c]));
                    }
                }
            }
        }
    }

    if settings.check_geometry {
        geometry.problems = look_for_trouble(&geometry.mesh);
    }
    geometry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map;

    fn cube_map(texture: &str) -> String {
        format!(
            r#"
{{
"classname" "worldspawn"
{{
( -32 -32 -32 ) ( -32 -31 -32 ) ( -32 -32 -31 ) {texture} [ 0 -1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -32 -32 -32 ) ( -32 -32 -31 ) ( -31 -32 -32 ) {texture} [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -32 -32 -32 ) ( -31 -32 -32 ) ( -32 -31 -32 ) {texture} [ -1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 32 32 32 ) ( 32 33 32 ) ( 33 32 32 ) {texture} [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 32 32 32 ) ( 33 32 32 ) ( 32 32 33 ) {texture} [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 32 32 32 ) ( 32 32 33 ) ( 32 33 32 ) {texture} [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
}}
}}
"#
        )
    }

    #[test]
    fn external_materials_sharing_a_texture_append_all_vertices_and_indices() {
        let surface = Surface {
            texture: "shared".into(),
            vertices: [
                Vec3::new(0., 0., 0.),
                Vec3::new(32., 0., 0.),
                Vec3::new(0., 32., 0.),
            ]
            .map(|position| Vertex {
                position,
                normal: Vec3::new(0., 0., 1.),
                uv: Vec2::new(0., 0.),
            })
            .to_vec(),
            indices: vec![0, 1, 2],
        };
        let mut entity = Entity::default();
        entity.external_surfaces = vec![surface.clone(), surface];
        let geo = build_solid(
            &entity,
            &Settings::default(),
            SolidOptions::default(),
            &TextureSizes::new(32.),
        );
        assert_eq!(geo.mesh.surfaces.len(), 1);
        assert_eq!(geo.mesh.surfaces[0].vertices.len(), 6);
        assert_eq!(geo.mesh.surfaces[0].indices, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_cube_makes_six_quads() {
        let map = map::parse(&cube_map("stone"), "t.map").unwrap();
        let settings = Settings::default();
        let textures = TextureSizes::new(32.0);
        let geo = build_solid(
            &map.entities[0],
            &settings,
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &textures,
        );

        assert_eq!(geo.mesh.surfaces.len(), 1);
        let surface = &geo.mesh.surfaces[0];
        assert_eq!(surface.vertices.len(), 24, "six quads of four vertices");
        assert_eq!(surface.indices.len(), 36, "six quads of two triangles");
        assert_eq!(geo.convex_hulls.len(), 1);
        assert_eq!(geo.convex_hulls[0].len(), 8);

        // 64 map units at 1/32 is 2 Godot units across, centred on the origin.
        for v in &surface.vertices {
            assert!((v.position.x.abs() - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn every_face_normal_points_away_from_the_centre() {
        let map = map::parse(&cube_map("stone"), "t.map").unwrap();
        let geo = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        for v in &geo.mesh.surfaces[0].vertices {
            assert!(
                v.position.dot(v.normal) > 0.0,
                "normal faces inward at {:?}",
                v.position
            );
        }
    }

    #[test]
    fn triangles_wind_clockwise_seen_from_outside() {
        let map = map::parse(&cube_map("stone"), "t.map").unwrap();
        let geo = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        let s = &geo.mesh.surfaces[0];
        for tri in s.indices.chunks(3) {
            let a = s.vertices[tri[0] as usize].position;
            let b = s.vertices[tri[1] as usize].position;
            let c = s.vertices[tri[2] as usize].position;
            let geometric = (b - a).cross(c - a).normalize();
            let shading = s.vertices[tri[0] as usize].normal;
            assert!(
                geometric.dot(shading) < 0.0,
                "expected clockwise winding around the outward normal"
            );
        }
    }

    #[test]
    fn valve_uvs_match_the_texture_grid() {
        let map = map::parse(&cube_map("stone"), "t.map").unwrap();
        let mut textures = TextureSizes::new(32.0);
        textures.insert("stone", Vec2::new(64.0, 64.0));
        let geo = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &textures,
        );
        // A 64 unit face on a 64 pixel texture at scale 1 spans exactly one tile.
        let uvs: Vec<Vec2> = geo.mesh.surfaces[0].vertices.iter().map(|v| v.uv).collect();
        let min = uvs.iter().fold(Vec2::splat(f64::MAX), |a, b| a.min(*b));
        let max = uvs.iter().fold(Vec2::splat(f64::MIN), |a, b| a.max(*b));
        assert!(
            (max.x - min.x - 1.0).abs() < 1e-9,
            "u span was {}",
            max.x - min.x
        );
        assert!(
            (max.y - min.y - 1.0).abs() < 1e-9,
            "v span was {}",
            max.y - min.y
        );
    }

    #[test]
    fn origin_brushes_are_left_out_of_geometry_and_set_the_pivot() {
        let mut text = String::from("{\n\"classname\" \"func_door\"\n");
        // Geometry cube from -32..32, plus an origin cube from 0..16.
        let cube = cube_map("stone");
        let brush = cube.lines().skip(3).take(8).collect::<Vec<_>>().join("\n");
        text.push_str(&brush);
        text.push('\n');
        text.push_str(
            r#"{
( 0 0 0 ) ( 0 1 0 ) ( 0 0 1 ) origin [ 0 -1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 0 0 0 ) ( 0 0 1 ) ( 1 0 0 ) origin [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 0 0 0 ) ( 1 0 0 ) ( 0 1 0 ) origin [ -1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 16 16 16 ) ( 16 17 16 ) ( 17 16 16 ) origin [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 16 16 16 ) ( 17 16 16 ) ( 16 16 17 ) origin [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 16 16 16 ) ( 16 16 17 ) ( 16 17 16 ) origin [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
}
}
"#,
        );

        let map = map::parse(&text, "t.map").unwrap();
        let geo = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions::default(),
            &TextureSizes::new(32.0),
        );
        assert_eq!(
            geo.origin,
            Vec3::splat(8.0),
            "pivot is the origin brush centre"
        );
        assert_eq!(
            geo.mesh.surfaces.len(),
            1,
            "the origin brush contributes no surface"
        );
        assert_eq!(geo.convex_hulls.len(), 1, "and no collision either");
    }

    #[test]
    fn faces_pressed_together_by_two_brushes_are_dropped() {
        // Two cubes side by side, sharing the plane at x = 32.
        let text = format!(
            "{{\n\"classname\" \"worldspawn\"\n\"_cull_interior_faces\" \"1\"\n{}\n{}\n}}\n",
            cube_brush(0.0, 0.0, 0.0, 32.0, 32.0, 32.0),
            cube_brush(32.0, 0.0, 0.0, 64.0, 32.0, 32.0)
        );
        let map = map::parse(&text, "t.map").unwrap();
        // Face joining is left out of it, or the two cubes would come out as
        // one box and there would be nothing to count.
        let plain = Settings {
            merge_faces: false,
            ..Default::default()
        };
        let culled = build_solid(
            &map.entities[0],
            &plain,
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );

        // Dropping them is what happens on its own, so the comparison has to
        // ask for the opposite out loud.
        let text_off = text.replace(
            "\"_cull_interior_faces\" \"1\"",
            "\"_cull_interior_faces\" \"0\"",
        );
        let map_off = map::parse(&text_off, "t.map").unwrap();
        let kept = build_solid(
            &map_off.entities[0],
            &plain,
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );

        // Twelve faces between them, two of which face each other.
        assert_eq!(kept.mesh.triangle_count(), 24);
        assert_eq!(
            culled.mesh.triangle_count(),
            20,
            "the shared pair goes away"
        );

        // Two cubes side by side make one box, so the physics engine is handed
        // one shape rather than two. The mesh is untouched by that.
        assert_eq!(culled.convex_hulls.len(), 1);
        assert_eq!(culled.convex_hulls[0].len(), 8, "the corners of the pair");
        assert_eq!(culled.merged_hulls, 1);

        let apart = build_solid(
            &map.entities[0],
            &Settings {
                merge_collision: false,
                merge_faces: false,
                ..Default::default()
            },
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert_eq!(
            apart.convex_hulls.len(),
            2,
            "one each when asked to keep them"
        );
    }

    /// Any vertex sitting in the middle of another triangle's edge, which is
    /// the shape a hairline crack comes in.
    fn t_junctions(mesh: &Mesh) -> usize {
        let mut found = 0;
        for surface in &mesh.surfaces {
            let corners: Vec<Vec3> = surface.vertices.iter().map(|v| v.position).collect();
            for tri in surface.indices.chunks(3) {
                // Only faces looking the same way can crack against each other.
                // Two faces back to back share every edge and none of it shows.
                let facing = surface.vertices[tri[0] as usize].normal;
                for edge in 0..3 {
                    let from = corners[tri[edge] as usize];
                    let to = corners[tri[(edge + 1) % 3] as usize];
                    let along = to - from;
                    let length = along.length();
                    if length < 1e-9 {
                        continue;
                    }
                    for (i, p) in corners.iter().enumerate() {
                        if tri.contains(&(i as u32))
                            || surface.vertices[i].normal.dot(facing) < 0.999
                        {
                            continue;
                        }
                        let t = (*p - from).dot(along) / (length * length);
                        if !(0.001..0.999).contains(&t) {
                            continue;
                        }
                        if (from + along * t).distance(*p) < 1e-6 {
                            found += 1;
                        }
                    }
                }
            }
        }
        found
    }

    #[test]
    fn a_join_never_leaves_a_corner_stranded_on_an_edge() {
        // The two below join; the block above cannot join either, so its corner is
        // left in the middle of the long face's top edge.
        let text = format!(
            "{{\n\"classname\" \"worldspawn\"\n{}\n{}\n{}\n}}\n",
            cube_brush(0.0, 0.0, 0.0, 32.0, 32.0, 32.0),
            cube_brush(32.0, 0.0, 0.0, 64.0, 32.0, 32.0),
            cube_brush(0.0, 0.0, 32.0, 16.0, 32.0, 64.0)
        );
        let map = map::parse(&text, "t.map").unwrap();
        let built = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert!(
            built.merged_faces > 0,
            "nothing joined, the check has no teeth"
        );
        assert_eq!(
            t_junctions(&built.mesh),
            0,
            "{} corners left stranded",
            t_junctions(&built.mesh)
        );

        // And the same over every shape the other tests here build, since a
        // join that strands a corner does it quietly.
        for (name, text) in [
            ("doorway", DOORWAY),
            ("ring", RING_OF_FOUR),
            ("casing", DOORWAY_WITH_CASING),
            ("box in two", BOX_IN_TWO_HALVES),
        ] {
            let map = map::parse(text, "t.map").unwrap();
            let built = build_solid(
                &map.entities[0],
                &Settings::default(),
                SolidOptions {
                    force_zero_origin: true,
                    ..Default::default()
                },
                &TextureSizes::new(32.0),
            );
            assert_eq!(t_junctions(&built.mesh), 0, "{name} left a corner stranded");
        }
    }

    #[test]
    fn the_checker_sees_a_face_drawn_from_behind() {
        // What `heron_brush check` is for. A mesh built the ordinary way is clean, and
        // one triangle turned round has to be reported, or the check is a
        // decoration.
        let map = map::parse(&cube_map("stone"), "t.map").unwrap();
        let mut built = build_solid(
            &map.entities[0],
            &Settings {
                check_geometry: true,
                ..Default::default()
            },
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert!(built.problems.is_empty(), "{:?}", built.problems);

        let indices = &mut built.mesh.surfaces[0].indices;
        indices.swap(1, 2);
        let said = look_for_trouble(&built.mesh);
        assert!(
            said.iter().any(|s| s.contains("drawn from behind")),
            "{said:?}"
        );
    }

    #[test]
    fn a_patch_bends_and_meets_its_neighbours() {
        // Three by three control points with the middle row lifted: the corners
        // stay put and the middle bulges up to half the control height, which is
        // what a quadratic bezier does.
        let text = "\
{
\"classname\" \"worldspawn\"
{
patchDef2
{
concrete
( 3 3 0 0 0 )
(
( ( -128 -128 0 0 0 ) ( -128 0 128 0 -1 ) ( -128 128 0 0 -2 ) )
( ( 0 -128 0 1 0 ) ( 0 0 128 1 -1 ) ( 0 128 0 1 -2 ) )
( ( 128 -128 0 2 0 ) ( 128 0 128 2 -1 ) ( 128 128 0 2 -2 ) )
)
}
}
}
";
        let map = map::parse(text, "p.map").unwrap();
        let patch = &map.entities[0].patches[0];
        assert_eq!((patch.width, patch.height), (3, 3));
        assert_eq!(patch.points.len(), 9);

        let built = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        // Six subdivisions each way: a grid of seven by seven corners, and two
        // triangles per cell.
        assert_eq!(built.mesh.triangle_count(), 72);
        let corners = &built.mesh.surfaces[0].vertices;
        assert_eq!(corners.len(), 49);

        // Godot's y is the map's z, and the map is divided by 32.
        let top = corners
            .iter()
            .map(|v| v.position.y)
            .fold(f64::MIN, f64::max);
        assert!((top - 2.0).abs() < 1e-6, "the bulge reaches {top}, not 2");
        let low = corners
            .iter()
            .map(|v| v.position.y)
            .fold(f64::MAX, f64::min);
        assert!(low.abs() < 1e-6, "the edges sit at {low}, not 0");

        // And it is a surface, so it can only be collided with as one.
        assert_eq!(built.concave_triangles.len() / 3, 0, "convex by default");
        let concave = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                collision: CollisionKind::Concave,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert_eq!(concave.concave_triangles.len() / 3, 72);

        // And wound like everything else, or it is drawn from behind and the
        // whole thing is invisible, which is exactly what happened first.
        let with_a_brush = format!(
            "{{\n\"classname\" \"worldspawn\"\n{}\n{}\n}}\n",
            cube_brush(0.0, 0.0, 0.0, 32.0, 32.0, 32.0),
            text.split_once('\n')
                .unwrap()
                .1
                .trim_end()
                .trim_end_matches('}')
        );
        let mixed = map::parse(&with_a_brush, "p.map").unwrap();
        let both = build_solid(
            &mixed.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        let (with, against) = winding_split(&both.mesh);
        assert!(
            with == 0 || against == 0,
            "patch and brush disagree: {with} one way, {against} the other"
        );
    }

    #[test]
    fn blocks_stacked_any_way_come_out_as_one_box() {
        // The square and the cube are the interesting ones: joining works on pairs,
        // and pairs have to keep finding each other after the first join.
        let mut cases: Vec<(String, Vec<String>)> = Vec::new();
        for count in [2usize, 3, 4] {
            for axis in 0..3 {
                let blocks: Vec<String> = (0..count)
                    .map(|i| {
                        let step = i as f64 * 32.0;
                        let (x, y, z) = match axis {
                            0 => (step, 0.0, 0.0),
                            1 => (0.0, step, 0.0),
                            _ => (0.0, 0.0, step),
                        };
                        cube_brush(x, y, z, x + 32.0, y + 32.0, z + 32.0)
                    })
                    .collect();
                cases.push((format!("{count} along axis {axis}"), blocks));
            }
        }
        let square: Vec<String> = (0..4)
            .map(|i| {
                let (x, y) = ((i % 2) as f64 * 32.0, (i / 2) as f64 * 32.0);
                cube_brush(x, y, 0.0, x + 32.0, y + 32.0, 32.0)
            })
            .collect();
        cases.push(("a square of four".into(), square));
        let cube: Vec<String> = (0..8)
            .map(|i| {
                let (x, y, z) = (
                    (i % 2) as f64 * 32.0,
                    ((i / 2) % 2) as f64 * 32.0,
                    (i / 4) as f64 * 32.0,
                );
                cube_brush(x, y, z, x + 32.0, y + 32.0, z + 32.0)
            })
            .collect();
        cases.push(("a cube of eight".into(), cube));

        for (what, blocks) in cases {
            let text = format!(
                "{{\n\"classname\" \"worldspawn\"\n{}\n}}\n",
                blocks.join("\n")
            );
            let map = map::parse(&text, "stack.map").unwrap();
            let concave = build_solid(
                &map.entities[0],
                &Settings::default(),
                SolidOptions {
                    force_zero_origin: true,
                    collision: CollisionKind::Concave,
                    ..Default::default()
                },
                &TextureSizes::new(32.0),
            );
            assert_eq!(
                concave.mesh.triangle_count(),
                12,
                "{what}: mesh has {}",
                concave.mesh.triangle_count()
            );
            assert_eq!(
                concave.concave_triangles.len() / 3,
                12,
                "{what}: collision has {}",
                concave.concave_triangles.len() / 3
            );

            let convex = build_solid(
                &map.entities[0],
                &Settings::default(),
                SolidOptions {
                    force_zero_origin: true,
                    collision: CollisionKind::Convex,
                    ..Default::default()
                },
                &TextureSizes::new(32.0),
            );
            assert_eq!(
                convex.convex_hulls.len(),
                1,
                "{what}: {} shapes",
                convex.convex_hulls.len()
            );
            assert_eq!(convex.convex_hulls[0].len(), 8, "{what}: eight corners");
        }
    }

    #[test]
    fn the_physics_engine_gets_the_same_box_the_eye_does() {
        // It used to be 48: the concave path walked faces that joining had already
        // thrown away, so the physics held four boxes inside each other.
        let blocks: Vec<String> = (0..4)
            .map(|i| {
                let x = i as f64 * 32.0;
                cube_brush(x, 0.0, 0.0, x + 32.0, 32.0, 32.0)
            })
            .collect();
        let text = format!(
            "{{\n\"classname\" \"worldspawn\"\n{}\n}}\n",
            blocks.join("\n")
        );
        let map = map::parse(&text, "row.map").unwrap();
        let concave = SolidOptions {
            force_zero_origin: true,
            collision: CollisionKind::Concave,
            ..Default::default()
        };

        let built = build_solid(
            &map.entities[0],
            &Settings::default(),
            concave,
            &TextureSizes::new(32.0),
        );
        assert_eq!(built.mesh.triangle_count(), 12, "one box to look at");
        assert_eq!(
            built.concave_triangles.len() / 3,
            12,
            "and one box to walk into, not {}",
            built.concave_triangles.len() / 3
        );

        let raw = build_solid(
            &map.entities[0],
            &Settings {
                cull_hidden: false,
                merge_faces: false,
                ..Default::default()
            },
            concave,
            &TextureSizes::new(32.0),
        );
        assert_eq!(raw.concave_triangles.len() / 3, 48, "four boxes untouched");
    }

    #[test]
    fn a_doorway_with_its_casing_is_as_small_as_it_can_be_per_texture() {
        // A hole in a wall with three strips lining the opening, so the frame can
        // carry its own texture. The strips cover the wall's inner faces exactly.
        //
        // Counted by hand, per material: six triangles a side for the C, five quads
        // round the rim. Two materials cannot share a surface, so 22 each is the
        // floor here.
        let map = map::parse(DOORWAY_WITH_CASING, "opt.map").unwrap();
        let built = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert_eq!(built.mesh.surfaces.len(), 2, "one surface per texture");
        for surface in &built.mesh.surfaces {
            assert_eq!(
                surface.indices.len() / 3,
                22,
                "{} came out at {} triangles",
                surface.texture,
                surface.indices.len() / 3
            );
        }

        let raw = build_solid(
            &map.entities[0],
            &Settings {
                cull_hidden: false,
                merge_faces: false,
                ..Default::default()
            },
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert_eq!(raw.mesh.triangle_count(), 72, "six brushes untouched");
    }

    /// A doorway lined with casing strips, wall and frame textured apart.
    const DOORWAY_WITH_CASING: &str = "{\n\"mapversion\" \"220\"\n\"classname\" \"worldspawn\"\n{\n( 416 48 80 ) ( 416 32 80 ) ( 416 32 0 ) dirt [ 0 -1 0 0 ] [ 0 0 -1 0 ] 180 1 1\n( 416 48 80 ) ( 496 48 128 ) ( 496 32 128 ) dirt [ 1 0 0 16 ] [ 0 -1 0 0 ] 180 1 1\n( 416 32 80 ) ( 496 32 128 ) ( 496 32 0 ) dirt [ 1 0 0 16 ] [ 0 0 -1 0 ] 180 1 1\n( 496 32 0 ) ( 496 48 0 ) ( 416 48 0 ) dirt [ -1 0 0 -16 ] [ 0 -1 0 0 ] 180 1 1\n( 496 48 0 ) ( 496 48 128 ) ( 416 48 80 ) dirt [ -1 0 0 -16 ] [ 0 0 -1 0 ] 180 1 1\n( 496 32 128 ) ( 496 48 128 ) ( 496 48 0 ) dirt [ 0 1 0 0 ] [ 0 0 -1 0 ] 180 1 1\n}\n{\n( 368 48 80 ) ( 288 48 128 ) ( 288 32 128 ) dirt [ 0 -1 0 0 ] [ -1 0 -2.220446049250313e-16 0 ] 180 1 1\n( 288 32 128 ) ( 496 32 128 ) ( 416 32 80 ) dirt [ 1 0 0 16 ] [ 0 0 -1 0 ] 180 1 1\n( 416 32 80 ) ( 416 48 80 ) ( 368 48 80 ) dirt [ -1 0 0 -16 ] [ 0 -1 0 0 ] 180 1 1\n( 288 48 128 ) ( 496 48 128 ) ( 496 32 128 ) dirt [ 1 0 0 16 ] [ 0 -1 0 0 ] 180 1 1\n( 416 48 80 ) ( 496 48 128 ) ( 288 48 128 ) dirt [ -1 0 0 -16 ] [ 0 0 -1 0 ] 180 1 1\n( 496 32 128 ) ( 496 48 128 ) ( 416 48 80 ) dirt [ 0 1 0 0 ] [ 1 0 -2.220446049250313e-16 16 ] 180 1 1\n}\n{\n( 288 48 128 ) ( 288 32 128 ) ( 288 32 0 ) dirt [ 0 -1 0 0 ] [ 0 0 -1 0 ] 180 1 1\n( 288 32 128 ) ( 368 32 80 ) ( 368 32 0 ) dirt [ 1 0 0 16 ] [ 0 0 -1 0 ] 180 1 1\n( 368 32 0 ) ( 368 48 0 ) ( 288 48 0 ) dirt [ -1 0 0 -16 ] [ 0 -1 0 0 ] 180 1 1\n( 368 48 0 ) ( 368 48 80 ) ( 288 48 128 ) dirt [ -1 0 0 -16 ] [ 0 0 -1 0 ] 180 1 1\n( 288 48 128 ) ( 368 48 80 ) ( 368 32 80 ) dirt [ 1 0 0 16 ] [ 0 -1 0 0 ] 180 1 1\n( 368 32 80 ) ( 368 48 80 ) ( 368 48 0 ) dirt [ 0 1 0 0 ] [ 0 0 -1 0 ] 180 1 1\n}\n{\n( 368 48 80 ) ( 368 32 80 ) ( 368 32 0 ) grass [ 0 -1 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( 368 32 80 ) ( 372 32 76 ) ( 372 32 0 ) grass [ 1 0 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( 372 32 0 ) ( 372 48 0 ) ( 368 48 0 ) grass [ -1 0 0 0 ] [ 0 -1 0 0 ] 90 1 1\n( 372 48 0 ) ( 372 48 76 ) ( 368 48 80 ) grass [ -1 0 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( 368 48 80 ) ( 372 48 76 ) ( 372 32 76 ) grass [ 1 0 0 0 ] [ 0 -1 0 0 ] 90 1 1\n( 372 32 76 ) ( 372 48 76 ) ( 372 48 0 ) grass [ 0 1 0 0 ] [ 0 0 -1 0 ] 90 1 1\n}\n{\n( 372 48 76 ) ( 368 48 80 ) ( 368 32 80 ) grass [ 0 -1 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( 368 32 80 ) ( 416 32 80 ) ( 412 32 76 ) grass [ 1 0 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( 412 32 76 ) ( 412 48 76 ) ( 372 48 76 ) grass [ -1 0 0 0 ] [ 0 -1 0 0 ] 90 1 1\n( 368 48 80 ) ( 416 48 80 ) ( 416 32 80 ) grass [ 1 0 0 0 ] [ 0 -1 0 0 ] 90 1 1\n( 412 48 76 ) ( 416 48 80 ) ( 368 48 80 ) grass [ -1 0 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( 416 32 80 ) ( 416 48 80 ) ( 412 48 76 ) grass [ 0 1 0 0 ] [ 0 0 -1 0 ] 90 1 1\n}\n{\n( 412 48 76 ) ( 412 32 76 ) ( 412 32 0 ) grass [ 0 -1 0 0 ] [ 0 0 -1 0 ] 270 1 1\n( 412 48 76 ) ( 416 48 80 ) ( 416 32 80 ) grass [ 1 0 0 20 ] [ 0 -1 0 0 ] 270 1 1\n( 412 32 76 ) ( 416 32 80 ) ( 416 32 0 ) grass [ 1 0 0 20 ] [ 0 0 -1 0 ] 270 1 1\n( 416 32 0 ) ( 416 48 0 ) ( 412 48 0 ) grass [ -1 0 0 -20 ] [ 0 -1 0 0 ] 270 1 1\n( 416 48 0 ) ( 416 48 80 ) ( 412 48 76 ) grass [ -1 0 0 -20 ] [ 0 0 -1 0 ] 270 1 1\n( 416 32 80 ) ( 416 48 80 ) ( 416 48 0 ) grass [ 0 1 0 0 ] [ 0 0 -1 0 ] 270 1 1\n}\n}\n";

    #[test]
    fn a_doorway_and_a_ring_come_out_as_small_as_a_modeller_would_draw_them() {
        // A doorway and a ring, both out of a real map. Neither can be one convex
        // piece and neither needs to be: triangles are corners less two, and a
        // mitred joint adds no corner. Counted by hand, 28 and 32.
        //
        // If somebody tries to make joining clever here, these numbers say what
        // there is to gain: nothing.
        for (name, text, triangles) in [("doorway", DOORWAY, 28), ("ring", RING_OF_FOUR, 32)] {
            let map = map::parse(text, "opt.map").unwrap();
            let built = build_solid(
                &map.entities[0],
                &Settings::default(),
                SolidOptions {
                    force_zero_origin: true,
                    ..Default::default()
                },
                &TextureSizes::new(32.0),
            );
            assert_eq!(
                built.mesh.triangle_count(),
                triangles,
                "{name} came out at {} triangles",
                built.mesh.triangle_count()
            );
        }
    }

    /// Two jambs and a lintel, mitred. Straight out of a map.
    const DOORWAY: &str = "{\n\"mapversion\" \"220\"\n\"classname\" \"worldspawn\"\n{\n( -304 48 128 ) ( -304 32 128 ) ( -304 32 0 ) __TB_empty [ 0 -1 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -304 32 128 ) ( -224 32 80 ) ( -224 32 0 ) __TB_empty [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -224 32 0 ) ( -224 48 0 ) ( -304 48 0 ) __TB_empty [ -1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1\n( -224 48 0 ) ( -224 48 80 ) ( -304 48 128 ) __TB_empty [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -304 48 128 ) ( -224 48 80 ) ( -224 32 80 ) __TB_empty [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1\n( -224 32 80 ) ( -224 48 80 ) ( -224 48 0 ) __TB_empty [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1\n}\n{\n( -176 48 80 ) ( -176 32 80 ) ( -176 32 0 ) __TB_empty [ 0 -1 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -176 48 80 ) ( -96 48 128 ) ( -96 32 128 ) __TB_empty [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1\n( -176 32 80 ) ( -96 32 128 ) ( -96 32 0 ) __TB_empty [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -96 32 0 ) ( -96 48 0 ) ( -176 48 0 ) __TB_empty [ -1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1\n( -96 48 0 ) ( -96 48 128 ) ( -176 48 80 ) __TB_empty [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -96 32 128 ) ( -96 48 128 ) ( -96 48 0 ) __TB_empty [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1\n}\n{\n( -224 48 80 ) ( -304 48 128 ) ( -304 32 128 ) __TB_empty [ 0 -1 0 0 ] [ -1 0 -2.220446049250313e-16 -16 ] 0 1 1\n( -304 32 128 ) ( -96 32 128 ) ( -176 32 80 ) __TB_empty [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -176 32 80 ) ( -176 48 80 ) ( -224 48 80 ) __TB_empty [ -1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1\n( -304 48 128 ) ( -96 48 128 ) ( -96 32 128 ) __TB_empty [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1\n( -176 48 80 ) ( -96 48 128 ) ( -304 48 128 ) __TB_empty [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -96 32 128 ) ( -96 48 128 ) ( -176 48 80 ) __TB_empty [ 0 1 0 0 ] [ 1 0 -2.220446049250313e-16 0 ] 0 1 1\n}\n}\n";

    /// Four bars mitred into a closed ring, which is a hole with a wall round
    /// it and cannot be convex at all.
    const RING_OF_FOUR: &str = "{\n\"mapversion\" \"220\"\n\"classname\" \"worldspawn\"\n{\n( -560 48 0 ) ( -576 64 0 ) ( -576 64 128 ) __TB_empty [ 0 -1 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -560 48 128 ) ( -528 48 128 ) ( -528 48 0 ) __TB_empty [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -528 48 0 ) ( -512 64 0 ) ( -576 64 0 ) __TB_empty [ -1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1\n( -576 64 128 ) ( -512 64 128 ) ( -528 48 128 ) __TB_empty [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1\n( -512 64 0 ) ( -512 64 128 ) ( -576 64 128 ) __TB_empty [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -528 48 128 ) ( -512 64 128 ) ( -512 64 0 ) __TB_empty [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1\n}\n{\n( -528 48 128 ) ( -528 16 128 ) ( -528 16 0 ) __TB_empty [ -1.0718754395722282e-15 -1 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -528 16 128 ) ( -512 0 128 ) ( -512 0 0 ) __TB_empty [ 1 -1.0718754395722282e-15 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -528 48 0 ) ( -512 64 0 ) ( -512 64 128 ) __TB_empty [ -1 1.0718754395722282e-15 0 0 ] [ 0 0 -1 0 ] 0 1 1\n( -528 16 0 ) ( -512 0 0 ) ( -512 64 0 ) __TB_empty [ 1.0718754395722282e-15 1 0 0 ] [ -1 1.0718754395722282e-15 0 0 ] 270 1 1\n( -512 64 128 ) ( -512 0 128 ) ( -528 16 128 ) __TB_empty [ -1.0718754395722282e-15 -1 0 0 ] [ -1 1.0718754395722282e-15 0 0 ] 90 1 1\n( -512 0 0 ) ( -512 0 128 ) ( -512 64 128 ) __TB_empty [ 1.0718754395722282e-15 1 0 0 ] [ 0 0 -1 0 ] 0 1 1\n}\n{\n( -576 64 0 ) ( -576 64 128 ) ( -576 0 128 ) __TB_empty [ -1.1943401194869635e-15 -1 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( -560 48 0 ) ( -576 64 0 ) ( -576 0 0 ) __TB_empty [ -1.1943401194869635e-15 -1 0 0 ] [ 1 -1.1943401194869635e-15 0 0 ] 0 1 1\n( -576 0 128 ) ( -576 64 128 ) ( -560 48 128 ) __TB_empty [ 1.1943401194869635e-15 1 0 0 ] [ 1 -1.1943401194869635e-15 0 0 ] 180 1 1\n( -560 16 0 ) ( -576 0 0 ) ( -576 0 128 ) __TB_empty [ 1 -1.1943401194869635e-15 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( -560 48 128 ) ( -576 64 128 ) ( -576 64 0 ) __TB_empty [ -1 1.1943401194869635e-15 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( -560 16 128 ) ( -560 48 128 ) ( -560 48 0 ) __TB_empty [ 1.1943401194869635e-15 1 0 0 ] [ 0 0 -1 0 ] 90 1 1\n}\n{\n( -560 16 128 ) ( -576 0 128 ) ( -576 0 0 ) __TB_empty [ -1.2246467991473532e-16 -1 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( -576 0 0 ) ( -576 0 128 ) ( -512 0 128 ) __TB_empty [ 1 -1.2246467991473532e-16 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( -560 16 0 ) ( -576 0 0 ) ( -512 0 0 ) __TB_empty [ 1 -1.2246467991473532e-16 0 0 ] [ 1.2246467991473532e-16 1 0 0 ] 90 1 1\n( -512 0 128 ) ( -576 0 128 ) ( -560 16 128 ) __TB_empty [ -1 1.2246467991473532e-16 0 0 ] [ 1.2246467991473532e-16 1 0 0 ] 90 1 1\n( -528 16 128 ) ( -560 16 128 ) ( -560 16 0 ) __TB_empty [ -1 1.2246467991473532e-16 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( -528 16 0 ) ( -512 0 0 ) ( -512 0 128 ) __TB_empty [ 1.2246467991473532e-16 1 0 0 ] [ 0 0 -1 0 ] 90 1 1\n}\n}\n";

    #[test]
    fn a_box_drawn_as_two_halves_comes_out_as_a_box() {
        // One block cut across at 45 degrees, so two brushes fill a plain box.
        //
        // What used to stop it joining was the rotation number: the editor wrote
        // 90 on one half and 270 on the other, and Valve 220 never reads it.
        let map = map::parse(BOX_IN_TWO_HALVES, "opt.map").unwrap();
        let options = SolidOptions {
            force_zero_origin: true,
            ..Default::default()
        };
        let built = build_solid(
            &map.entities[0],
            &Settings::default(),
            options,
            &TextureSizes::new(32.0),
        );
        assert_eq!(built.mesh.triangle_count(), 12, "a box is twelve triangles");

        let raw = build_solid(
            &map.entities[0],
            &Settings {
                cull_hidden: false,
                merge_faces: false,
                ..Default::default()
            },
            options,
            &TextureSizes::new(32.0),
        );
        assert_eq!(raw.mesh.triangle_count(), 24, "two brushes untouched");
    }

    /// A box cut across at 45 degrees, drawn in an editor as two brushes.
    const BOX_IN_TWO_HALVES: &str = "{\n\"mapversion\" \"220\"\n\"classname\" \"worldspawn\"\n{\n( -416 32 128 ) ( -416 0 128 ) ( -416 0 0 ) __TB_empty [ -1.0718754395722282e-15 -1 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( -400 48 0 ) ( -400 48 128 ) ( -416 32 128 ) __TB_empty [ -1 1.0718754395722282e-15 0 16 ] [ 0 0 -1 0 ] 90 1 1\n( -416 0 128 ) ( -400 0 128 ) ( -400 0 0 ) __TB_empty [ 1 -1.0718754395722282e-15 0 -16 ] [ 0 0 -1 0 ] 90 1 1\n( -400 0 0 ) ( -400 48 0 ) ( -416 32 0 ) __TB_empty [ 1.0718754395722282e-15 1 0 0 ] [ -1 1.0718754395722282e-15 0 16 ] 0 1 1\n( -416 32 128 ) ( -400 48 128 ) ( -400 0 128 ) __TB_empty [ -1.0718754395722282e-15 -1 0 0 ] [ -1 1.0718754395722282e-15 0 16 ] 180 1 1\n( -400 0 128 ) ( -400 48 128 ) ( -400 48 0 ) __TB_empty [ 1.0718754395722282e-15 1 0 0 ] [ 0 0 -1 0 ] 90 1 1\n}\n{\n( -416 80 128 ) ( -416 32 128 ) ( -416 32 0 ) __TB_empty [ -1.0718754395722282e-15 -1 0 0 ] [ 0 0 -1 0 ] 270 1 1\n( -400 48 0 ) ( -400 80 0 ) ( -416 80 0 ) __TB_empty [ 1.0718754395722282e-15 1 0 0 ] [ -1 1.0718754395722282e-15 0 16 ] 180 1 1\n( -416 80 128 ) ( -400 80 128 ) ( -400 48 128 ) __TB_empty [ -1.0718754395722282e-15 -1 0 0 ] [ -1 1.0718754395722282e-15 0 16 ] 0 1 1\n( -400 80 0 ) ( -400 80 128 ) ( -416 80 128 ) __TB_empty [ -1 1.0718754395722282e-15 0 16 ] [ 0 0 -1 0 ] 270 1 1\n( -416 32 128 ) ( -400 48 128 ) ( -400 48 0 ) __TB_empty [ -9.43689570931383e-16 -1 0 -16 ] [ 0 0 -1 0 ] 270 1 1\n( -400 48 128 ) ( -400 80 128 ) ( -400 80 0 ) __TB_empty [ 1.0718754395722282e-15 1 0 0 ] [ 0 0 -1 0 ] 270 1 1\n}\n}\n";

    #[test]
    fn two_wall_pieces_mitred_together_lose_the_face_between_them() {
        // A mitred corner. Both halves of the diagonal are inside solid wall.
        let text = "\
{
\"classname\" \"worldspawn\"
\"mapversion\" \"220\"
{
( -144 448 616 ) ( -160 464 616 ) ( -160 464 688 ) concrete [ -1 0 0 28 ] [ 0 0 -1 0 ] 270 1 1
( -144 448 688 ) ( 0 448 688 ) ( 0 448 616 ) concrete [ 1 0 0 16 ] [ 0 0 -1 0 ] 180 1 1
( 0 448 616 ) ( 16 464 616 ) ( -160 464 616 ) concrete [ 1 0 0 16 ] [ 0 -1 0 20 ] 180 1 1
( -160 464 688 ) ( 16 464 688 ) ( 0 448 688 ) concrete [ 1 0 0 16 ] [ 0 -1 0 20 ] 180 1 1
( 16 464 616 ) ( 16 464 688 ) ( -160 464 688 ) concrete [ 1 0 0 16 ] [ 0 0 -1 0 ] 180 1 1
( 0 448 688 ) ( 16 464 688 ) ( 16 464 616 ) concrete [ 0 1 0 -20 ] [ 0 0 -1 0 ] 90 1 1
}
{
( -160 464 688 ) ( -160 220 688 ) ( -160 220 616 ) concrete [ 0 1 0 0 ] [ 0 0 -1 0 ] 270 1 1
( -160 220 688 ) ( -144 220 688 ) ( -144 220 616 ) concrete [ 1 0 0 0 ] [ 0 0 -1 0 ] 90 1 1
( -144 220 616 ) ( -144 448 616 ) ( -160 464 616 ) concrete [ 1 0 0 0 ] [ 0 -1 0 0 ] 270 1 1
( -160 464 688 ) ( -144 448 688 ) ( -144 220 688 ) concrete [ 1 0 0 0 ] [ 0 -1 0 0 ] 90 1 1
( -144 448 616 ) ( -144 448 688 ) ( -160 464 688 ) concrete [ 0 -1 0 48 ] [ 0 0 -1 0 ] 0 1 1
( -144 220 688 ) ( -144 448 688 ) ( -144 448 616 ) concrete [ 0 1 0 0 ] [ 0 0 -1 0 ] 90 1 1
}
}
";
        let map = map::parse(text, "mitre.map").unwrap();
        let options = SolidOptions {
            force_zero_origin: true,
            ..Default::default()
        };
        let joined = build_solid(
            &map.entities[0],
            &Settings::default(),
            options,
            &TextureSizes::new(32.0),
        );
        let kept = build_solid(
            &map.entities[0],
            &Settings {
                cull_hidden: false,
                ..Default::default()
            },
            options,
            &TextureSizes::new(32.0),
        );
        // Two quads, one on each piece, so four triangles.
        assert_eq!(
            kept.mesh.triangle_count() - joined.mesh.triangle_count(),
            4,
            "{} against {}",
            joined.mesh.triangle_count(),
            kept.mesh.triangle_count()
        );

        // And with teeth: an entity that asks for its brushes verbatim keeps
        // them, because the switch is an override and not decoration.
        let text_off = text.replace(
            "\"mapversion\" \"220\"",
            "\"mapversion\" \"220\"\n\"_cull_interior_faces\" \"0\"",
        );
        let asked_to_keep = build_solid(
            &map::parse(&text_off, "mitre.map").unwrap().entities[0],
            &Settings::default(),
            options,
            &TextureSizes::new(32.0),
        );
        assert_eq!(
            asked_to_keep.mesh.triangle_count(),
            kept.mesh.triangle_count()
        );

        // The same corner turned off every axis, so nothing lines up with the grid.
        // The rule is same corners, opposite normals, whatever the angle.
        let turned = ROTATED_MITRE;
        let turned_map = map::parse(turned, "mitre.map").unwrap();
        let turned_joined = build_solid(
            &turned_map.entities[0],
            &Settings::default(),
            options,
            &TextureSizes::new(32.0),
        );
        let turned_kept = build_solid(
            &turned_map.entities[0],
            &Settings {
                cull_hidden: false,
                ..Default::default()
            },
            options,
            &TextureSizes::new(32.0),
        );
        assert_eq!(
            turned_kept.mesh.triangle_count() - turned_joined.mesh.triangle_count(),
            4,
            "turned corner kept its inside faces, {} against {}",
            turned_joined.mesh.triangle_count(),
            turned_kept.mesh.triangle_count()
        );
    }

    /// The mitred corner above, rotated off every axis.
    const ROTATED_MITRE: &str = "{\n\"classname\" \"worldspawn\"\n\"mapversion\" \"220\"\n{\n( -331.692683 186.671264 675.299787 ) ( -353.212636 193.484309 676.872703 ) ( -353.212636 177.287833 747.027348 ) concrete [ -1 2.220446049250313e-16 0 28 ] [ 0 0 -1 0 ] 270 1 1\n( -331.692683 170.474788 745.454432 ) ( -203.387744 234.173873 760.160525 ) ( -203.387744 250.370348 690.005880 ) concrete [ 1 0 0 16 ] [ 0 0 -1 0 ] 180 1 1\n( -203.387744 250.370348 690.005880 ) ( -196.395487 271.338746 694.846816 ) ( -353.212636 193.484309 676.872703 ) concrete [ 1 0 0 16 ] [ 0 -1 0 20 ] 180 1 1\n( -353.212636 177.287833 747.027348 ) ( -196.395487 255.142270 765.001461 ) ( -203.387744 234.173873 760.160525 ) concrete [ 1 0 0 16 ] [ 0 -1 0 20 ] 180 1 1\n( -196.395487 271.338746 694.846816 ) ( -196.395487 255.142270 765.001461 ) ( -353.212636 177.287833 747.027348 ) concrete [ 1 0 0 16 ] [ 0 0 -1 0 ] 180 1 1\n( -203.387744 234.173873 760.160525 ) ( -196.395487 255.142270 765.001461 ) ( -196.395487 271.338746 694.846816 ) concrete [ 0 1 0 -20 ] [ 0 0 -1 0 ] 90 1 1\n}\n{\n( -353.212636 177.287833 747.027348 ) ( -242.438954 -34.545667 698.121730 ) ( -242.438954 -18.349191 627.967086 ) concrete [ 0 1 0 0 ] [ 0 0 -1 0 ] 270 1 1\n( -242.438954 -34.545667 698.121730 ) ( -228.182849 -27.467991 699.755741 ) ( -228.182849 -11.271515 629.601096 ) concrete [ 1 0 0 0 ] [ 0 0 -1 0 ] 90 1 1\n( -228.182849 -11.271515 629.601096 ) ( -331.692683 186.671264 675.299787 ) ( -353.212636 193.484309 676.872703 ) concrete [ 1 0 0 0 ] [ 0 -1 0 0 ] 270 1 1\n( -353.212636 177.287833 747.027348 ) ( -331.692683 170.474788 745.454432 ) ( -228.182849 -27.467991 699.755741 ) concrete [ 1 0 0 0 ] [ 0 -1 0 0 ] 90 1 1\n( -331.692683 186.671264 675.299787 ) ( -331.692683 170.474788 745.454432 ) ( -353.212636 177.287833 747.027348 ) concrete [ 2.220446049250313e-16 -1 0 48 ] [ 0 0 -1 0 ] 0 1 1\n( -228.182849 -27.467991 699.755741 ) ( -331.692683 170.474788 745.454432 ) ( -331.692683 186.671264 675.299787 ) concrete [ 0 1 0 0 ] [ 0 0 -1 0 ] 90 1 1\n}\n}\n";

    #[test]
    fn a_row_of_copied_blocks_comes_out_as_one_face_each_way() {
        // What a mapper gets by copying one block along a wall.
        let blocks: Vec<String> = (0..4)
            .map(|i| {
                let x = i as f64 * 32.0;
                cube_brush(x, 0.0, 0.0, x + 32.0, 32.0, 32.0)
            })
            .collect();
        let text = format!(
            "{{\n\"classname\" \"worldspawn\"\n{}\n}}\n",
            blocks.join("\n")
        );
        let map = map::parse(&text, "t.map").unwrap();

        let joined = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        let apart = build_solid(
            &map.entities[0],
            &Settings {
                merge_faces: false,
                ..Default::default()
            },
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert!(joined.merged_faces >= 9, "{} joined", joined.merged_faces);
        assert!(
            joined.mesh.triangle_count() < apart.mesh.triangle_count(),
            "{} against {}",
            joined.mesh.triangle_count(),
            apart.mesh.triangle_count()
        );
        // The row is 128 long and 32 high, and the outside of it has not moved.
        assert_eq!(
            joined.bounds.map(|b| b.max.x),
            apart.bounds.map(|b| b.max.x)
        );
    }

    /// Triangles wound with their normal, and against it. The axis swizzle turns
    /// every one of them round, so the whole mesh should land in one pile.
    fn winding_split(mesh: &Mesh) -> (usize, usize) {
        let (mut with, mut against) = (0, 0);
        for surface in &mesh.surfaces {
            for tri in surface.indices.chunks(3) {
                let a = surface.vertices[tri[0] as usize];
                let b = surface.vertices[tri[1] as usize];
                let c = surface.vertices[tri[2] as usize];
                let wound = (b.position - a.position).cross(c.position - a.position);
                if wound.length() < 1e-9 {
                    continue;
                }
                if wound.normalize().dot(a.normal) < 0.0 {
                    against += 1;
                } else {
                    with += 1;
                }
            }
        }
        (with, against)
    }

    /// Total area of every triangle drawn.
    fn drawn_area(mesh: &Mesh) -> f64 {
        let mut area = 0.0;
        for surface in &mesh.surfaces {
            for tri in surface.indices.chunks(3) {
                let a = surface.vertices[tri[0] as usize].position;
                let b = surface.vertices[tri[1] as usize].position;
                let c = surface.vertices[tri[2] as usize].position;
                area += (b - a).cross(c - a).length() * 0.5;
            }
        }
        area
    }

    #[test]
    fn a_joined_face_still_faces_the_way_it_did() {
        // A hull comes out wound whichever way the hull code likes. Against the
        // face's own normal, the wall is drawn only from behind.
        let blocks: Vec<String> = (0..3)
            .map(|i| {
                let x = i as f64 * 32.0;
                cube_brush(x, 0.0, 0.0, x + 32.0, 32.0, 32.0)
            })
            .collect();
        let text = format!(
            "{{\n\"classname\" \"worldspawn\"\n{}\n}}\n",
            blocks.join("\n")
        );
        let map = map::parse(&text, "t.map").unwrap();
        let geo = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert!(
            geo.merged_faces > 0,
            "nothing merged, the check has no teeth"
        );

        let apart = build_solid(
            &map.entities[0],
            &Settings {
                merge_faces: false,
                ..Default::default()
            },
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        let (with_apart, against_apart) = winding_split(&apart.mesh);
        assert!(
            with_apart == 0 || against_apart == 0,
            "unmerged mesh is already mixed, {with_apart} against {against_apart}"
        );
        let (with, against) = winding_split(&geo.mesh);
        assert!(
            with == 0 || against == 0,
            "{with} triangles one way and {against} the other after joining"
        );
        assert_eq!(
            with == 0,
            with_apart == 0,
            "joining turned the mesh round as a whole"
        );

        // Fewer triangles covering exactly as much. Anything else is a hole.
        let joined_area = drawn_area(&geo.mesh);
        let apart_area = drawn_area(&apart.mesh);
        assert!(
            (joined_area - apart_area).abs() < apart_area * 1e-6,
            "{joined_area} drawn after joining against {apart_area} before"
        );
        assert!(geo.mesh.triangle_count() < apart.mesh.triangle_count());
    }

    #[test]
    fn only_walls_worth_the_processor_become_occluders() {
        // A big block and a small one. An occluder is drawn on the processor
        // every frame, so a hundred small ones cost more than they save.
        let text = format!(
            "{{\n\"classname\" \"worldspawn\"\n{}\n{}\n}}\n",
            cube_brush(0.0, 0.0, 0.0, 256.0, 16.0, 256.0),
            cube_brush(0.0, 64.0, 0.0, 32.0, 80.0, 32.0)
        );
        let map = map::parse(&text, "t.map").unwrap();
        let with_a_limit = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                // Nothing smaller than 100 by 100.
                occluder_min_area: 100.0 * 100.0,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        // Only the big block's two 256 by 256 sides are over the limit.
        assert_eq!(with_a_limit.occluder_faces.len(), 2);

        let asking_for_none = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert!(
            asking_for_none.occluder_faces.is_empty(),
            "off unless a size is given"
        );
    }

    #[test]
    fn blocks_of_different_heights_keep_their_sides() {
        // A step. The faces that would make an L must stay as they are.
        let text = format!(
            "{{\n\"classname\" \"worldspawn\"\n{}\n{}\n}}\n",
            cube_brush(0.0, 0.0, 0.0, 32.0, 32.0, 32.0),
            cube_brush(32.0, 0.0, 0.0, 64.0, 32.0, 64.0)
        );
        let map = map::parse(&text, "t.map").unwrap();
        let geo = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        // The bottoms make a rectangle and join. The sides make an L, and joining
        // those would draw a lid over the step. One join, not three.
        assert_eq!(geo.merged_faces, 1, "the bottoms and nothing else");
    }

    #[test]
    fn nothing_is_dropped_when_brushes_only_meet_at_an_edge() {
        let text = format!(
            "{{\n\"classname\" \"worldspawn\"\n\"_cull_interior_faces\" \"1\"\n{}\n{}\n}}\n",
            cube_brush(0.0, 0.0, 0.0, 32.0, 32.0, 32.0),
            cube_brush(32.0, 32.0, 0.0, 64.0, 64.0, 32.0)
        );
        let map = map::parse(&text, "t.map").unwrap();
        let geo = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert_eq!(
            geo.mesh.triangle_count(),
            24,
            "touching along an edge buries nothing"
        );
    }

    /// An axis aligned box brush, written the way an editor writes one.
    fn cube_brush(x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64) -> String {
        let faces = [
            ((x0, y0, z0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)),
            ((x1, y0, z0), (0.0, 0.0, 1.0), (0.0, 1.0, 0.0)),
            ((x0, y0, z0), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0)),
            ((x0, y1, z0), (1.0, 0.0, 0.0), (0.0, 0.0, 1.0)),
            ((x0, y0, z0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)),
            ((x0, y0, z1), (0.0, 1.0, 0.0), (1.0, 0.0, 0.0)),
        ];
        let mut out = String::from("{\n");
        for (p, a, b) in faces {
            out.push_str(&format!(
                "( {} {} {} ) ( {} {} {} ) ( {} {} {} ) stone [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1\n",
                p.0,
                p.1,
                p.2,
                p.0 + a.0,
                p.1 + a.1,
                p.2 + a.2,
                p.0 + b.0,
                p.1 + b.1,
                p.2 + b.2
            ));
        }
        out.push('}');
        out
    }

    #[test]
    fn a_brush_with_a_flipped_face_is_reported_rather_than_drawn() {
        // The same box, but the +y face written the wrong way round.
        let good = cube_brush(0.0, 0.0, 0.0, 32.0, 32.0, 32.0);
        let bad = good.replace(
            "( 0 32 0 ) ( 1 32 0 ) ( 0 32 1 )",
            "( 0 32 0 ) ( 0 32 1 ) ( 1 32 0 )",
        );
        assert_ne!(good, bad, "the test rewrote the face it meant to");

        let text = format!("{{\n\"classname\" \"worldspawn\"\n{bad}\n}}\n");
        let map = map::parse(&text, "t.map").unwrap();
        let geo = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert_eq!(geo.unbounded_brushes, vec![0], "the open brush is reported");

        // Whatever survives stays near the brush instead of running to the
        // edge of the world.
        for surface in &geo.mesh.surfaces {
            for v in &surface.vertices {
                assert!(
                    v.position.length() < 100.0,
                    "stray vertex at {:?}",
                    v.position
                );
            }
        }
    }

    #[test]
    fn a_normal_brush_is_closed() {
        let text = format!(
            "{{\n\"classname\" \"worldspawn\"\n{}\n}}\n",
            cube_brush(0.0, 0.0, 0.0, 32.0, 32.0, 32.0)
        );
        let map = map::parse(&text, "t.map").unwrap();
        let geo = build_solid(
            &map.entities[0],
            &Settings::default(),
            SolidOptions {
                force_zero_origin: true,
                ..Default::default()
            },
            &TextureSizes::new(32.0),
        );
        assert!(geo.unbounded_brushes.is_empty());
        assert_eq!(geo.mesh.triangle_count(), 12);
    }

    #[test]
    fn standard_format_floors_get_the_classic_axes() {
        let (u, v) = standard_axes(Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(u, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(v, Vec3::new(0.0, -1.0, 0.0));
    }
}
