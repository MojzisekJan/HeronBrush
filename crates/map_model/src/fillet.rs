//! Rounding the edges of a brush.
//!
//! A fillet replaces a sharp brush edge with a segmented rounded surface of a specified radius.
//!
//! A brush is a convex solid, and a convex solid is the space behind a set of planes, so
//! rounding an edge here is not a mesh operation at all: it is a handful of extra planes.
//! Picture a cylinder of the chosen radius laid into the corner, touching both faces along
//! its length. Every plane that touches that cylinder cuts off part of the edge and none of
//! the solid the cylinder sits in, and a fan of them across the corner leaves a run of
//! narrow faces following the cylinder — the fillet, at as many faces as were asked for.
//! One face is a chamfer; eight is hard to tell from round.
//!
//! Working in planes is what keeps this cheap and keeps the result a brush: no vertex
//! stitching, no new kind of geometry, and every face still a flat polygon a `.map` file
//! can hold. It also settles the corners for free. Rounding several edges at once takes
//! every plane from the brush as it was and applies them together, so where two rounded
//! edges meet, their planes cut each other and leave a mitre.

use geometry_math::plane::Plane3d;
use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vec3d};

use crate::brush::{Brush, ClipOutcome};
use crate::polyhedron::{EdgeId, Polyhedron, VertexId};

/// How close two positions must be to be the same end of an edge.
///
/// An edge is named by the positions of its ends, which have been through a rotation or a
/// drag by the time they come back here, so they are matched with the tolerance the rest
/// of the editor picks vertices at rather than exactly.
const EDGE_MATCH_TOLERANCE: f64 = 0.01;

/// What to do where two or three rounded edges meet at a vertex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Corners {
    /// Let the edges' own planes cut each other, which leaves a mitre: two flat faces
    /// meeting in a line, the way a picture frame's corner is cut.
    #[default]
    Mitred,
    /// Round the vertex as well, so the corner is part of a sphere of the same radius.
    /// Three edges of a cube meeting at a corner leave an eighth of a ball.
    Rounded,
    /// Cut the vertex off with one flat face across the corner, touching the ball
    /// [`Corners::Rounded`] would lay there.
    Chamfered,
    /// Round the vertex with a ball [`SETBACK`] times the radius, so the corner starts
    /// further back along each edge than the edges' own rounding and blends into it
    /// gently rather than meeting it at the vertex.
    Setback,
}

/// How much larger than the edges' radius a [`Corners::Setback`] corner's ball is.
pub const SETBACK: f64 = 2.0;

/// What a fillet is: how far back it starts, how many faces it is made of, and what
/// happens where two of them meet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fillet {
    /// How far back from the edge the rounding starts.
    pub radius: f64,
    /// How many flat faces each edge's rounding is made of. One is a chamfer.
    pub faces: usize,
    /// What to do at a vertex where rounded edges meet.
    pub corners: Corners,
}

impl Default for Fillet {
    fn default() -> Self {
        Self {
            radius: 8.0,
            faces: 4,
            corners: Corners::default(),
        }
    }
}

/// Why an edge could not be rounded.
#[derive(Debug, Clone, PartialEq)]
pub enum FilletError {
    /// The brush has no edge between the given positions.
    NoSuchEdge {
        /// One end of the edge that was asked for.
        from: Vec3d,
        /// The other end.
        to: Vec3d,
    },
    /// A radius of zero or less, which would round nothing.
    RadiusNotPositive {
        /// What was asked for.
        given: f64,
    },
    /// Too many planes for an interactive fillet.
    TooManyFaces {
        /// Requested segment count.
        given: usize,
    },
    /// No segment was requested.
    TooFewFaces {
        /// How many were asked for.
        given: usize,
    },
    /// The two faces meeting at the edge are as good as in one plane, so there is no
    /// corner there to round.
    EdgeIsFlat {
        /// One end of the edge.
        from: Vec3d,
        /// The other end.
        to: Vec3d,
    },
    /// The radius is larger than the brush can give up: rounding at it would cut the
    /// brush away entirely.
    RadiusTooLarge {
        /// What was asked for.
        given: f64,
    },
}

impl std::fmt::Display for FilletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilletError::NoSuchEdge { from, to } => {
                write!(f, "the brush has no edge from {from:?} to {to:?}")
            }
            FilletError::RadiusNotPositive { given } => {
                write!(f, "a fillet radius must be more than zero, not {given}")
            }
            FilletError::TooManyFaces { given } => write!(f,"a fillet supports at most 64 faces, not {given}"),
            FilletError::TooFewFaces { given } => {
                write!(f, "a fillet takes at least one face, not {given}")
            }
            FilletError::EdgeIsFlat { from, to } => {
                write!(f, "the edge from {from:?} to {to:?} has no corner to round")
            }
            FilletError::RadiusTooLarge { given } => {
                write!(f, "a radius of {given} would cut the whole brush away")
            }
        }
    }
}

impl std::error::Error for FilletError {}

/// Rounds one edge of `brush`, given by the positions of its two ends.
///
/// `faces` is how many flat faces the rounding is made of: one gives a chamfer, more
/// approach a quarter cylinder of the given radius.
pub fn fillet_edge(
    brush: &Brush,
    from: Vec3d,
    to: Vec3d,
    radius: f64,
    faces: usize,
) -> Result<Brush, FilletError> {
    fillet_edges(
        brush,
        &[(from, to)],
        &Fillet {
            radius,
            faces,
            corners: Corners::Mitred,
        },
    )
}

/// Rounds several edges of `brush` at once, each at the same radius.
///
/// The planes for every edge are worked out from the brush as it stands before any of them
/// is applied, so an edge is rounded against the corner it actually has rather than one an
/// earlier rounding has already cut into. Applying them together is also what mitres the
/// place where two rounded edges meet.
pub fn fillet_edges(
    brush: &Brush,
    edges: &[(Vec3d, Vec3d)],
    fillet: &Fillet,
) -> Result<Brush, FilletError> {
    let (radius, faces) = (fillet.radius, fillet.faces);
    if !radius.is_finite() || radius <= 0.0 {
        return Err(FilletError::RadiusNotPositive { given: radius });
    }
    if faces < 1 {
        return Err(FilletError::TooFewFaces { given: faces });
    }

    if faces>64 {return Err(FilletError::TooManyFaces{given:faces});}
    let geometry = brush.geometry();
    let mut strips=Vec::new();let mut used=std::collections::HashSet::new();let mut unique_edges=Vec::new();
    let mut planes = Vec::with_capacity(edges.len() * faces);
    for &(from, to) in edges {
        let edge = find_edge(geometry, from, to)
            .ok_or(FilletError::NoSuchEdge { from, to })?;
        if !used.insert(edge.0) {continue;}unique_edges.push((from,to));
        let arc=edge_planes(geometry,edge,radius,faces).ok_or(FilletError::EdgeIsFlat{from,to})?;
        let edge=geometry.edge(edge);
        let sources=[edge.first,edge.second].map(|half| {
            let normal=geometry.face_plane(geometry.half_edge(half).face).normal;
            brush.faces().iter().find(|face|vec::length(face.normal()-normal)<1e-7).unwrap().clone()
        });
        strips.push((sources,arc.clone(),from));
        planes.extend(arc);
    }

    if fillet.corners != Corners::Mitred {
        planes.extend(corner_planes(geometry, &unique_edges, radius, faces, fillet.corners));
    }

    let mut rounded = brush.clone();
    for plane in planes {
        match rounded.clip(&plane) {
            ClipOutcome::Clipped(brush) => rounded = brush,
            ClipOutcome::Unchanged => {}
            ClipOutcome::Empty => {
                return Err(FilletError::RadiusTooLarge { given: radius })
            }
        }
    }
    let mut attrs=rounded.faces().to_vec();
    for (sources,arc,from) in strips {
        let same=sources[0].material_name==sources[1].material_name;
        for reverse in [false,true] {
            if same && reverse {continue;}
            let mut previous=sources[usize::from(reverse)].clone();
            let planes:Vec<_>=if reverse {arc.iter().rev().collect()} else {arc.iter().collect()};
            let indices:Vec<_>=planes.iter().filter_map(|p|attrs.iter().position(|f|vec::length(f.normal()-p.normal)<1e-7 && (f.plane.distance-p.distance).abs()<1e-6)).collect();
            for index in indices {
                if !same && attrs[index].material_name!=previous.material_name {continue;}
                carry_fillet_uv(&previous,&mut attrs[index],from);previous=attrs[index].clone();
            }
            if same {
                if let Some(index)=attrs.iter().position(|f|vec::length(f.normal()-sources[1].normal())<1e-7 && (f.plane.distance-sources[1].plane.distance).abs()<1e-6) {carry_fillet_uv(&previous,&mut attrs[index],from);}
            }
        }
    }
    Ok(rounded.with_exact_face_attributes(&attrs).unwrap())
}

fn carry_fillet_uv(source:&crate::brush_face::BrushFace,target:&mut crate::brush_face::BrushFace,near:Vec3d) {
    let p=source.plane;let q=target.plane;let line=vec::cross(p.normal,q.normal);let denominator=vec::squared_length(line);
    let anchor=if denominator>1e-12 {
        (vec::cross(q.normal,line)*p.distance+vec::cross(line,p.normal)*q.distance+line*vec::dot(near,line))/denominator
    } else {near};
    target.wrap_uv_from(source,anchor);
}

/// Returns the edges around one face of a brush, as pairs of end positions.
///
/// This is what rounds a whole face's outline in one go: the loop of edges where the face
/// meets its neighbours.
pub fn face_edges(brush: &Brush, face: crate::polyhedron::FaceId) -> Vec<(Vec3d, Vec3d)> {
    let geometry = brush.geometry();
    geometry
        .face_half_edges(face)
        .into_iter()
        .map(|half_edge| {
            let origin = geometry.vertex(geometry.half_edge(half_edge).origin).position;
            let destination = geometry.vertex(geometry.destination(half_edge)).position;
            (origin, destination)
        })
        .collect()
}

/// Whether the brush has an edge between the two given positions.
///
/// Several brushes can be selected at once while only some of them have the edge that was
/// picked, so this is how a caller sorts out which of them a fillet is for.
pub fn has_edge(brush: &Brush, from: Vec3d, to: Vec3d) -> bool {
    find_edge(brush.geometry(), from, to).is_some()
}

/// Finds the edge whose ends are at the two given positions, in either order.
fn find_edge(geometry: &Polyhedron<f64>, from: Vec3d, to: Vec3d) -> Option<EdgeId> {
    (0..geometry.edge_count() as u32).map(EdgeId).find(|&id| {
        let half_edge = geometry.edge(id).first;
        let origin = geometry.vertex(geometry.half_edge(half_edge).origin).position;
        let destination = geometry.vertex(geometry.destination(half_edge)).position;
        (is_at(origin, from) && is_at(destination, to))
            || (is_at(origin, to) && is_at(destination, from))
    })
}

/// Whether two positions are the same end of an edge.
fn is_at(position: Vec3d, wanted: Vec3d) -> bool {
    vec::is_zero(position - wanted, EDGE_MATCH_TOLERANCE)
}

/// The planes that round the vertices where two or three of the given edges meet.
///
/// A mitre is what the edges' own planes leave on their own: two flat faces meeting in a
/// line. Rounding the corner as well means laying a *ball* of the same radius into it,
/// where each edge laid in a cylinder, and cutting with planes that touch that ball. The
/// centre is the point that is one radius inside every face meeting at the vertex, which
/// is the same condition the cylinder's axis satisfied for two.
///
/// The normals are the spherical triangle between the three faces' own normals,
/// subdivided as finely as the face count asks: at one face that is the three corners and
/// the middle, which already cuts the corner off; at four it is a ball. A chamfer takes
/// the middle alone, and a setback the same spread round a larger ball.
fn corner_planes(
    geometry: &Polyhedron<f64>,
    edges: &[(Vec3d, Vec3d)],
    radius: f64,
    faces: usize,
    corners: Corners,
) -> Vec<Plane3d> {
    let radius = if corners == Corners::Setback { radius * SETBACK } else { radius };
    let mut planes = Vec::new();
    for vertex in shared_vertices(geometry, edges) {
        // Every face meeting there. Three is the case a corner of a box gives, and the
        // one a sphere is the right answer for; anything else is left to the mitre, since
        // a point where four faces meet has no single ball touching all of them.
        let normals: Vec<Vec3d> = {
            let mut seen: Vec<Vec3d> = Vec::new();
            for face in geometry.vertex_faces(vertex) {
                let normal = geometry.face_plane(face).normal;
                if !seen
                    .iter()
                    .any(|already| vec::is_zero(*already - normal, f64::ALMOST_ZERO))
                {
                    seen.push(normal);
                }
            }
            seen
        };
        if normals.len() != 3 {
            continue;
        }

        let position = geometry.vertex(vertex).position;
        let Some(center) = ball_center(geometry, vertex, &normals, radius) else {
            continue;
        };
        // A centre that has wandered further than the radius from the vertex means the
        // corner is too shallow for a ball that size; the mitre is the better answer.
        if vec::length(center - position) > radius * 8.0 {
            continue;
        }

        let normals = if corners == Corners::Chamfered {
            vec![vec::normalize(normals[0] + normals[1] + normals[2])]
        } else {
            spread(&normals, faces)
        };
        for normal in normals {
            planes.push(Plane3d::new(vec::dot(normal, center) + radius, normal));
        }
    }
    planes
}

/// The vertices two or more of the given edges share.
fn shared_vertices(geometry: &Polyhedron<f64>, edges: &[(Vec3d, Vec3d)]) -> Vec<VertexId> {
    let mut counted: Vec<(VertexId, usize)> = Vec::new();
    for &(from, to) in edges {
        let Some(edge) = find_edge(geometry, from, to) else {
            continue;
        };
        let half = geometry.edge(edge).first;
        for vertex in [
            geometry.half_edge(half).origin,
            geometry.destination(half),
        ] {
            match counted.iter_mut().find(|(seen, _)| *seen == vertex) {
                Some((_, count)) => *count += 1,
                None => counted.push((vertex, 1)),
            }
        }
    }
    counted
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(vertex, _)| vertex)
        .collect()
}

/// The centre of the ball that touches all three faces at a vertex from inside.
///
/// Each face wants `n·c = d - r`, which is three equations in three unknowns; Cramer's
/// rule solves them, and a corner whose three faces are nearly in one plane has no
/// solution worth having, which the determinant says.
fn ball_center(
    geometry: &Polyhedron<f64>,
    vertex: VertexId,
    normals: &[Vec3d],
    radius: f64,
) -> Option<Vec3d> {
    let mut wanted = [0.0f64; 3];
    let mut rows = [Vec3d::zero(); 3];
    let mut at = 0;
    for face in geometry.vertex_faces(vertex) {
        let plane = geometry.face_plane(face);
        if !normals
            .iter()
            .any(|normal| vec::is_zero(*normal - plane.normal, f64::ALMOST_ZERO))
        {
            continue;
        }
        // Each face once: `vertex_faces` walks round the vertex and a face can come back.
        if rows[..at]
            .iter()
            .any(|already| vec::is_zero(*already - plane.normal, f64::ALMOST_ZERO))
        {
            continue;
        }
        rows[at] = plane.normal;
        wanted[at] = plane.distance - radius;
        at += 1;
        if at == 3 {
            break;
        }
    }
    if at < 3 {
        return None;
    }

    let determinant = |a: Vec3d, b: Vec3d, c: Vec3d| vec::dot(a, vec::cross(b, c));
    let whole = determinant(rows[0], rows[1], rows[2]);
    if whole.abs() < f64::ALMOST_ZERO {
        return None;
    }

    // Cramer's rule: each coordinate is the determinant with that column replaced by the
    // right-hand side, over the whole determinant.
    let replaced = |column: usize| {
        let mut with = rows;
        for (row, value) in with.iter_mut().zip(wanted) {
            row[column] = value;
        }
        determinant(with[0], with[1], with[2])
    };
    Some(Vec3d::new(
        replaced(0) / whole,
        replaced(1) / whole,
        replaced(2) / whole,
    ))
}

/// Normals spread over the spherical triangle between three, at the given fineness.
///
/// The triangle is subdivided as a flat one would be -- every point that is a whole number
/// of steps along two of its sides -- and each point is pushed out to the sphere. At one
/// step that is the three corners; at four it is fifteen normals, which reads as a ball.
fn spread(normals: &[Vec3d], faces: usize) -> Vec<Vec3d> {
    let steps = faces.max(1);
    let mut spread = Vec::new();
    for i in 0..=steps {
        for j in 0..=(steps - i) {
            let k = steps - i - j;
            let mixed = normals[0] * i as f64 + normals[1] * j as f64 + normals[2] * k as f64;
            if vec::is_zero(mixed, f64::ALMOST_ZERO) {
                continue;
            }
            spread.push(vec::normalize(mixed));
        }
    }
    spread
}

/// The planes that round one edge, or `None` if its two faces are as good as coplanar.
///
/// The two faces meet at some angle, and their outward normals turn through the same
/// angle across the corner. The cylinder laid into the corner has its axis where both
/// faces are one radius away; a plane touching it at any turn of that angle is the plane
/// with that normal, one radius out from the axis. Taking the fan at the middle of each of
/// `faces` equal steps puts every plane tangent to the cylinder, so the rounding sits just
/// outside it rather than cutting into it.
fn edge_planes(
    geometry: &Polyhedron<f64>,
    edge: EdgeId,
    radius: f64,
    faces: usize,
) -> Option<Vec<Plane3d>> {
    let edge = geometry.edge(edge);
    let first = geometry.half_edge(edge.first);
    let second = geometry.half_edge(edge.second);
    let one = geometry.face_plane(first.face);
    let other = geometry.face_plane(second.face);

    let turn = vec::dot(one.normal, other.normal).clamp(-1.0, 1.0);
    // Coplanar faces have no corner; faces folded back on each other cannot happen in a
    // convex solid, and the arithmetic below would divide by nothing if they did.
    if turn > 1.0 - f64::ALMOST_ZERO || turn < -1.0 + f64::ALMOST_ZERO {
        return None;
    }

    // The axis of the cylinder in the corner: the point a radius behind both faces. Going
    // back along both normals by the same amount lands there, and that amount falls out of
    // asking for both distances to be the radius at once.
    let step_back = radius / (1.0 + turn);
    let on_edge = geometry.vertex(first.origin).position;
    let axis = on_edge - (one.normal + other.normal) * step_back;

    // The normals to sweep between, as an angle and the direction the turn is towards.
    let angle = turn.acos();
    let across = vec::normalize(other.normal - one.normal * turn);

    Some(
        (0..faces)
            .map(|face| {
                let along = angle * (2 * face + 1) as f64 / (2 * faces) as f64;
                let normal = one.normal * along.cos() + across * along.sin();
                Plane3d::new(vec::dot(normal, axis) + radius, normal)
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::bbox::Bbox3d;

    /// The world the test brushes are built in.
    fn world() -> Bbox3d {
        Bbox3d::new(Vec3d::fill(-16384.0), Vec3d::fill(16384.0))
    }

    /// A cube from the origin to `size` on every axis.
    fn cube(size: f64) -> Brush {
        crate::shapes::create_cuboid(
            &world(),
            &Bbox3d::new(Vec3d::zero(), Vec3d::fill(size)),
            "test",
        )
        .expect("the cube is built")
    }

    /// The top edge of the cube running along Y at x = size.
    fn top_edge(size: f64) -> (Vec3d, Vec3d) {
        (
            Vec3d::new(size, 0.0, size),
            Vec3d::new(size, size, size),
        )
    }

    #[test]
    fn one_face_chamfers_the_edge() {
        let brush = cube(64.0);
        let (from, to) = top_edge(64.0);
        let rounded = fillet_edge(&brush, from, to, 8.0, 1).expect("the edge rounds");

        assert_eq!(rounded.geometry().face_count(), 7);
        let chamfer = rounded
            .face_planes()
            .into_iter()
            .find(|plane| plane.normal.z() > 0.1 && plane.normal.x() > 0.1)
            .expect("the chamfer is there");
        // Halfway between the two faces it replaces, which for a right angle is 45
        // degrees on both.
        assert!((chamfer.normal.x() - 0.5f64.sqrt()).abs() < 1e-9);
        assert!((chamfer.normal.z() - 0.5f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn every_face_of_the_fillet_touches_the_same_cylinder() {
        let size = 64.0;
        let radius = 12.0;
        let brush = cube(size);
        let (from, to) = top_edge(size);
        let rounded = fillet_edge(&brush, from, to, radius, 6).expect("the edge rounds");

        assert_eq!(rounded.geometry().face_count(), 6 + 6);

        // The cylinder sits a radius inside both of the faces that met at the edge, and
        // runs along Y. The two ends of the brush are square to it, so they say nothing
        // about the rounding; every other face is measured against the axis.
        let axis = Vec3d::new(size - radius, 0.0, size - radius);
        let along_the_axis = rounded
            .face_planes()
            .into_iter()
            .filter(|plane| plane.normal.y().abs() < 1e-9);
        let mut touching = 0;
        for plane in along_the_axis {
            let distance = plane.distance - vec::dot(plane.normal, axis);
            // Nothing cuts into the cylinder, and what lies on it exactly is the six
            // faces the fillet added plus the two the cylinder was laid against.
            assert!(distance > radius - 1e-9, "a face cuts into the fillet");
            if (distance - radius).abs() < 1e-9 {
                touching += 1;
            }
        }
        assert_eq!(touching, 6 + 2);
    }

    #[test]
    fn rounding_takes_a_bite_out_of_the_brush() {
        let brush = cube(64.0);
        let (from, to) = top_edge(64.0);
        let rounded = fillet_edge(&brush, from, to, 8.0, 4).expect("the edge rounds");

        // The corner the fillet removed is the only thing that moved: the brush still
        // reaches the same box, and the far corner is still solid.
        assert_eq!(rounded.bounds(), brush.bounds());
        assert!(rounded.contains_point(Vec3d::new(1.0, 32.0, 1.0)));
        // A point just inside the sharp corner is now outside.
        assert!(brush.contains_point(Vec3d::new(63.0, 32.0, 63.0)));
        assert!(!rounded.contains_point(Vec3d::new(63.0, 32.0, 63.0)));
    }

    #[test]
    fn a_whole_face_can_be_rounded_at_once() {
        let size = 64.0;
        let brush = cube(size);
        let top = brush
            .find_face_by_normal(Vec3d::new(0.0, 0.0, 1.0), 1e-6)
            .and_then(|face| brush.geometry_face_of(face))
            .expect("the cube has a top");
        let edges = face_edges(&brush, top);
        assert_eq!(edges.len(), 4);

        let rounded = fillet_edges(&brush, &edges, &Fillet { radius: 8.0, faces: 2, corners: Corners::Mitred }).expect("the loop rounds");
        assert_eq!(rounded.geometry().face_count(), 6 + 4 * 2);
        // The top face is still there, narrower by the radius on all four sides.
        assert!(rounded.contains_point(Vec3d::new(32.0, 32.0, size - 0.5)));
        assert!(!rounded.contains_point(Vec3d::new(1.0, 1.0, size - 0.5)));
    }

    #[test]
    fn two_edges_that_meet_are_rounded_into_one_another() {
        let size = 64.0;
        let brush = cube(size);
        // The two top edges that meet at the corner (size, size, size).
        let edges = [
            (
                Vec3d::new(size, 0.0, size),
                Vec3d::new(size, size, size),
            ),
            (
                Vec3d::new(0.0, size, size),
                Vec3d::new(size, size, size),
            ),
        ];
        let rounded = fillet_edges(&brush, &edges, &Fillet { radius: 8.0, faces: 3, corners: Corners::Mitred }).expect("both edges round");

        // Six faces, three for each rounding, and no extra face where they meet: the two
        // sets of planes cut each other into a mitre rather than leaving a step.
        assert_eq!(rounded.geometry().face_count(), 6 + 3 + 3);
        // The corner they shared is gone, and the middle of the top is untouched.
        assert!(!rounded.contains_point(Vec3d::new(63.0, 63.0, 63.0)));
        assert!(rounded.contains_point(Vec3d::new(32.0, 32.0, size - 0.5)));
    }

    #[test]
    fn three_edges_meeting_at_a_vertex_round_the_corner() {
        let size = 64.0;
        let brush = cube(size);
        let corner = Vec3d::new(size, size, size);
        let edges = [
            (Vec3d::new(size, 0.0, size), corner),
            (Vec3d::new(0.0, size, size), corner),
            (Vec3d::new(size, size, 0.0), corner),
        ];
        let rounded = fillet_edges(&brush, &edges, &Fillet { radius: 8.0, faces: 2, corners: Corners::Mitred }).expect("the corner rounds");

        assert_eq!(rounded.geometry().face_count(), 6 + 3 * 2);
        assert!(!rounded.contains_point(Vec3d::new(63.0, 63.0, 63.0)));
        // The three faces that met there are still faces, just smaller.
        for normal in [
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
            Vec3d::new(0.0, 0.0, 1.0),
        ] {
            assert!(
                rounded.find_face_by_normal(normal, 1e-6).is_some(),
                "the {normal:?} face survived"
            );
        }
    }

    #[test]
    fn a_rounded_corner_is_part_of_a_ball() {
        let size = 64.0;
        let radius = 12.0;
        let brush = cube(size);
        let corner = Vec3d::new(size, size, size);
        let edges = [
            (Vec3d::new(size, 0.0, size), corner),
            (Vec3d::new(0.0, size, size), corner),
            (Vec3d::new(size, size, 0.0), corner),
        ];

        let mitred = fillet_edges(
            &brush,
            &edges,
            &Fillet {
                radius,
                faces: 2,
                corners: Corners::Mitred,
            },
        )
        .expect("the corner rounds");
        let rounded = fillet_edges(
            &brush,
            &edges,
            &Fillet {
                radius,
                faces: 2,
                corners: Corners::Rounded,
            },
        )
        .expect("the corner rounds");

        assert!(
            rounded.geometry().face_count() > mitred.geometry().face_count(),
            "rounding the corner adds faces to the mitre's"
        );

        // The ball sits a radius inside all three faces that met at the corner. Nothing
        // cuts into it, and the faces the corner added lie on it.
        let center = Vec3d::fill(size - radius);
        let mut touching = 0;
        for plane in rounded.face_planes() {
            let distance = plane.distance - vec::dot(plane.normal, center);
            assert!(distance > radius - 1e-9, "a face cuts into the ball");
            if (distance - radius).abs() < 1e-9 {
                touching += 1;
            }
        }
        assert!(touching > 6, "only {touching} faces lie on the ball");

        // The corner itself is gone, and further along each edge the rounding is the
        // cylinder's rather than the ball's, so the brush is not simply smaller.
        assert!(!rounded.contains_point(Vec3d::new(63.0, 63.0, 63.0)));
        assert!(rounded.contains_point(Vec3d::new(size - 1.0, 32.0, 32.0)));
    }

    /// The cube's corner at (64, 64, 64), rounded on its three edges with `corners`.
    fn round_corner(corners: Corners) -> Brush {
        let size = 64.0;
        let corner = Vec3d::new(size, size, size);
        let edges = [
            (Vec3d::new(size, 0.0, size), corner),
            (Vec3d::new(0.0, size, size), corner),
            (Vec3d::new(size, size, 0.0), corner),
        ];
        fillet_edges(&cube(size), &edges, &Fillet { radius: 12.0, faces: 2, corners }).expect("the corner rounds")
    }

    #[test]
    fn a_chamfered_corner_is_one_face_across_the_corner() {
        let chamfered = round_corner(Corners::Chamfered);
        let diagonal = vec::normalize(Vec3d::fill(1.0));
        let face = &chamfered.faces()[chamfered.find_face_by_normal(diagonal, 1e-9).expect("a face looks along the diagonal")];
        // It touches the ball the rounded corner lies on, a radius inside each face.
        let center = Vec3d::fill(64.0 - 12.0);
        assert!((face.plane.distance - vec::dot(diagonal, center) - 12.0).abs() < 1e-9);
        assert_eq!(
            chamfered.geometry().face_count(),
            round_corner(Corners::Mitred).geometry().face_count() + 1,
            "the chamfer is the mitre and one face"
        );
    }

    #[test]
    fn a_setback_corner_starts_further_back_than_the_rounded_one() {
        let rounded = round_corner(Corners::Rounded);
        let setback = round_corner(Corners::Setback);
        // Its ball is larger and sits deeper, so it cuts away all the rounded corner does
        // and more, while far along an edge both are the edge's own rounding.
        let center = Vec3d::fill(64.0 - 12.0 * SETBACK);
        for plane in setback.face_planes() {
            assert!(plane.distance - vec::dot(plane.normal, center) > 12.0 * SETBACK - 1e-9, "a face cuts into the ball");
        }
        let near = Vec3d::fill(58.0);
        assert!(rounded.contains_point(near) && !setback.contains_point(near));
        let along = Vec3d::new(63.0, 32.0, 32.0);
        assert!(rounded.contains_point(along) && setback.contains_point(along));
    }

    #[test]
    fn a_mitred_corner_is_what_it_was_before_the_choice_existed() {
        let brush = cube(64.0);
        let (from, to) = top_edge(64.0);
        let one = fillet_edge(&brush, from, to, 8.0, 3).expect("it rounds");
        let other = fillet_edges(
            &brush,
            &[(from, to)],
            &Fillet {
                radius: 8.0,
                faces: 3,
                corners: Corners::Mitred,
            },
        )
        .expect("it rounds");
        assert_eq!(
            one.geometry().face_count(),
            other.geometry().face_count(),
            "the default is the mitre"
        );
    }

    #[test]
    fn an_edge_that_is_not_there_is_an_error() {
        let brush = cube(64.0);
        let outcome = fillet_edge(
            &brush,
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(64.0, 64.0, 64.0),
            8.0,
            2,
        );
        assert!(matches!(outcome, Err(FilletError::NoSuchEdge { .. })));
    }

    #[test]
    fn a_radius_the_brush_cannot_give_up_is_an_error() {
        let brush = cube(64.0);
        let (from, to) = top_edge(64.0);
        let outcome = fillet_edge(&brush, from, to, 512.0, 4);
        assert!(matches!(outcome, Err(FilletError::RadiusTooLarge { .. })));
    }

    #[test]
    fn a_radius_of_nothing_and_a_fillet_of_no_faces_are_errors() {
        let brush = cube(64.0);
        let (from, to) = top_edge(64.0);
        assert!(matches!(
            fillet_edge(&brush, from, to, 0.0, 4),
            Err(FilletError::RadiusNotPositive { .. })
        ));
        assert!(matches!(
            fillet_edge(&brush, from, to, 8.0, 0),
            Err(FilletError::TooFewFaces { .. })
        ));
    }
}
