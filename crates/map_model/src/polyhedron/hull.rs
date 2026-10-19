//! Building a [`Polyhedron`] as the convex hull of a set of points.
//!
//! Builds a triangle hull and converts it to a half-edge mesh through
//! [`Polyhedron::from_faces`](super::Polyhedron::from_faces).
//!
//! This is the operation behind every edit that moves geometry rather than planes —
//! dragging a vertex, an edge or a face, and snapping vertices to the grid. Those all work
//! by moving points and asking what convex solid they now describe.
//!
//! The algorithm is the standard incremental one: start with a tetrahedron, then for each
//! remaining point, delete the faces it can see and stitch it to the horizon those faces
//! leave behind.

use rustc_hash::{FxHashMap, FxHashSet};

use geometry_math::plane::{self, Plane3d};
use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vec3d};

use super::{BuildError, Polyhedron};

/// Why a convex hull could not be built.
#[derive(Debug, Clone, PartialEq)]
pub enum HullError {
    /// Fewer than four distinct points were given.
    NotEnoughPoints {
        /// How many distinct points there were.
        distinct: usize,
    },
    /// Every point lies on one plane, so they enclose no volume.
    Coplanar,
    /// The faces the hull produced do not form a valid mesh.
    ///
    /// This means the points are very nearly degenerate — almost coplanar, or with two
    /// points almost on top of each other — in a way the epsilon did not catch.
    Degenerate(BuildError),
}

impl std::fmt::Display for HullError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HullError::NotEnoughPoints { distinct } => write!(
                f,
                "a convex hull needs at least 4 distinct points, got {distinct}"
            ),
            HullError::Coplanar => {
                f.write_str("all the points lie on one plane, so they enclose no volume")
            }
            HullError::Degenerate(cause) => {
                write!(f, "the hull's faces do not form a polyhedron: {cause}")
            }
        }
    }
}

impl std::error::Error for HullError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HullError::Degenerate(cause) => Some(cause),
            _ => None,
        }
    }
}

impl Polyhedron<f64> {
    /// Builds the convex hull of the given points.
    ///
    /// Points closer together than `epsilon` are treated as one, and points inside the
    /// hull are dropped. Coplanar faces are merged, so the hull of a cube's eight corners
    /// has six quadrilateral faces rather than twelve triangles.
    pub fn from_points(points: &[Vec3d], epsilon: f64) -> Result<Self, HullError> {
        let points = distinct_points(points, epsilon);
        if points.len() < 4 {
            return Err(HullError::NotEnoughPoints {
                distinct: points.len(),
            });
        }

        let triangles = build_hull(&points, epsilon)?;
        let faces = merge_coplanar(&points, triangles, epsilon);

        let polygons: Vec<Vec<Vec3d>> = faces
            .into_iter()
            .map(|ring| ring.into_iter().map(|i| points[i]).collect())
            .collect();

        Polyhedron::from_faces(&polygons, epsilon).map_err(HullError::Degenerate)
    }
}

/// Removes points that duplicate an earlier one within `epsilon`.
fn distinct_points(points: &[Vec3d], epsilon: f64) -> Vec<Vec3d> {
    let mut result: Vec<Vec3d> = Vec::with_capacity(points.len());
    for point in points {
        let duplicate = result
            .iter()
            .any(|kept| vec::squared_distance(*kept, *point) <= epsilon * epsilon);
        if !duplicate {
            result.push(*point);
        }
    }
    result
}

/// One triangular face of the hull under construction.
///
/// The vertices are wound clockwise as seen from outside, so `plane` — computed the way
/// [`geometry_math::plane::from_points`] computes it — has an outward normal.
#[derive(Debug, Clone, Copy)]
struct Triangle {
    vertices: [usize; 3],
    plane: Plane3d,
}

impl Triangle {
    /// The triangle's three directed edges, in winding order.
    fn edges(&self) -> [(usize, usize); 3] {
        let [a, b, c] = self.vertices;
        [(a, b), (b, c), (c, a)]
    }
}

/// Builds the hull as a set of triangles.
fn build_hull(points: &[Vec3d], epsilon: f64) -> Result<Vec<Triangle>, HullError> {
    let (seed, seed_plane) = initial_tetrahedron(points, epsilon)?;
    let mut triangles = seed;

    for (index, point) in points.iter().enumerate() {
        if seed_plane.contains(&index) {
            continue;
        }

        // A point is outside the hull exactly when some face has it in front.
        let visible: Vec<usize> = triangles
            .iter()
            .enumerate()
            .filter(|(_, t)| t.plane.point_distance(*point) > epsilon)
            .map(|(i, _)| i)
            .collect();
        if visible.is_empty() {
            // Inside the hull so far, so it is not a hull vertex.
            continue;
        }

        // The horizon is the boundary of the visible region: a directed edge of a visible
        // face whose reverse belongs to a face that is not visible.
        let visible_set: FxHashSet<usize> = visible.iter().copied().collect();
        let visible_edges: FxHashSet<(usize, usize)> = visible
            .iter()
            .flat_map(|i| triangles[*i].edges())
            .collect();

        let horizon: Vec<(usize, usize)> = visible
            .iter()
            .flat_map(|i| triangles[*i].edges())
            .filter(|(a, b)| !visible_edges.contains(&(*b, *a)))
            .collect();

        // Drop the faces the point can see, keeping the indices stable while filtering.
        triangles = triangles
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !visible_set.contains(i))
            .map(|(_, t)| t)
            .collect();

        // Stitch the point to the horizon. Keeping the edge's own direction means the new
        // face pairs with the face still on the other side of it.
        for (a, b) in horizon {
            if let Some(triangle) = make_triangle(points, [a, b, index]) {
                triangles.push(triangle);
            }
        }
    }

    if triangles.len() < 4 {
        return Err(HullError::Coplanar);
    }
    Ok(triangles)
}

/// Builds a triangle, or returns `None` if its points are colinear.
fn make_triangle(points: &[Vec3d], vertices: [usize; 3]) -> Option<Triangle> {
    let plane = plane::from_points(
        points[vertices[0]],
        points[vertices[1]],
        points[vertices[2]],
    )?;
    Some(Triangle { vertices, plane })
}

/// Finds four points that enclose a volume and returns the tetrahedron they make.
///
/// Also returns which points went into it, so the main loop can skip them.
fn initial_tetrahedron(
    points: &[Vec3d],
    epsilon: f64,
) -> Result<(Vec<Triangle>, FxHashSet<usize>), HullError> {
    // Two distinct points.
    let (a, b) = (0, 1);

    // A third that is not on the line through them.
    let c = (2..points.len())
        .find(|&i| !vec::is_colinear(points[a], points[b], points[i], f64::COLINEAR_EPSILON))
        .ok_or(HullError::Coplanar)?;

    let base = plane::from_points(points[a], points[b], points[c]).ok_or(HullError::Coplanar)?;

    // A fourth that is off that plane. Taking the one furthest off it rather than the
    // first gives the initial tetrahedron the most volume it can have, which keeps the
    // face planes well conditioned for the visibility tests that follow.
    let d = (0..points.len())
        .filter(|&i| i != a && i != b && i != c)
        .max_by(|&x, &y| {
            let dx = base.point_distance(points[x]).abs();
            let dy = base.point_distance(points[y]).abs();
            dx.partial_cmp(&dy).unwrap_or(std::cmp::Ordering::Equal)
        })
        .filter(|&i| base.point_distance(points[i]).abs() > epsilon)
        .ok_or(HullError::Coplanar)?;

    // Wind the base so the fourth point is behind it, then join it to each base edge.
    let base_ring = if base.point_distance(points[d]) > 0.0 {
        [a, c, b]
    } else {
        [a, b, c]
    };

    let mut triangles = Vec::with_capacity(4);
    triangles.push(make_triangle(points, base_ring).ok_or(HullError::Coplanar)?);
    for i in 0..3 {
        let (from, to) = (base_ring[(i + 1) % 3], base_ring[i]);
        triangles.push(make_triangle(points, [from, to, d]).ok_or(HullError::Coplanar)?);
    }

    let used = FxHashSet::from_iter([a, b, c, d]);
    Ok((triangles, used))
}

/// How close two planes must be to count as the same, when merging coplanar faces.
const COPLANAR_NORMAL_TOLERANCE: f64 = 1e-6;
/// The distance tolerance used alongside [`COPLANAR_NORMAL_TOLERANCE`].
const COPLANAR_DISTANCE_TOLERANCE: f64 = 1e-6;

/// Merges triangles that share a plane into single polygonal faces.
///
/// Without this the hull of a box would have twelve triangular faces rather than six
/// quadrilateral ones, and every edit would multiply a brush's face count.
fn merge_coplanar(
    points: &[Vec3d],
    triangles: Vec<Triangle>,
    epsilon: f64,
) -> Vec<Vec<usize>> {
    // Group the triangles by the plane they lie in.
    let mut groups: Vec<(Plane3d, Vec<Triangle>)> = Vec::new();
    for triangle in triangles {
        let group = groups.iter_mut().find(|(plane, _)| {
            vec::dot(plane.normal, triangle.plane.normal) > 1.0 - COPLANAR_NORMAL_TOLERANCE
                && (plane.distance - triangle.plane.distance).abs()
                    < COPLANAR_DISTANCE_TOLERANCE
        });
        match group {
            Some((_, members)) => members.push(triangle),
            None => groups.push((triangle.plane, vec![triangle])),
        }
    }

    groups
        .into_iter()
        .filter_map(|(_, members)| merge_group(points, &members, epsilon))
        .collect()
}

/// Chains the outer boundary of a group of coplanar triangles into one ring.
///
/// An edge shared by two triangles of the group is interior and is dropped; what is left
/// is the group's outline, and each of its vertices has exactly one outgoing edge.
fn merge_group(points: &[Vec3d], triangles: &[Triangle], epsilon: f64) -> Option<Vec<usize>> {
    if triangles.len() == 1 {
        return Some(triangles[0].vertices.to_vec());
    }

    let all_edges: FxHashSet<(usize, usize)> =
        triangles.iter().flat_map(Triangle::edges).collect();

    let mut next: FxHashMap<usize, usize> = FxHashMap::default();
    for (a, b) in &all_edges {
        if !all_edges.contains(&(*b, *a)) {
            next.insert(*a, *b);
        }
    }
    if next.is_empty() {
        return None;
    }

    // Walk the boundary from any vertex on it.
    let start = *next.keys().next()?;
    let mut ring = vec![start];
    let mut current = *next.get(&start)?;
    while current != start {
        ring.push(current);
        current = *next.get(&current)?;
        // A malformed boundary would loop forever otherwise.
        if ring.len() > next.len() {
            return None;
        }
    }

    Some(drop_colinear(points, ring, epsilon))
}

/// Removes vertices that lie on the straight line between their neighbours.
///
/// Merging triangles can leave a vertex in the middle of what is now a straight edge; it
/// is not a corner of the face and would make the mesh's edge pairing inconsistent.
fn drop_colinear(points: &[Vec3d], ring: Vec<usize>, epsilon: f64) -> Vec<usize> {
    if ring.len() <= 3 {
        return ring;
    }

    let mut result: Vec<usize> = Vec::with_capacity(ring.len());
    for i in 0..ring.len() {
        let previous = points[ring[(i + ring.len() - 1) % ring.len()]];
        let current = points[ring[i]];
        let next = points[ring[(i + 1) % ring.len()]];

        // The cross product of the two edges vanishes exactly when they are parallel.
        let area = vec::length(vec::cross(current - previous, next - current));
        if area > epsilon {
            result.push(ring[i]);
        }
    }

    if result.len() >= 3 {
        result
    } else {
        ring
    }
}
