//! Clipping a [`Polyhedron`] with a plane.
//!
//! This is the operation brushes are built from: a brush is defined by a set of planes,
//! and its geometry is the world box clipped by each plane in turn.
//!
//! Clips each face polygon independently and builds a new mesh from the results, leaving the
//! original mesh unchanged if construction fails.

use geometry_math::intersection;
use geometry_math::plane::Plane;
use geometry_math::scalar::Float;
use geometry_math::util::PlaneStatus;
use geometry_math::vec::{self, Vector};

use super::{BuildError, FaceId, Polyhedron};

/// What clipping a polyhedron with a plane did to it.
///
/// This deliberately does not implement [`PartialEq`]: two polyhedra can describe the
/// same solid while numbering their vertices differently, so there is no one answer to
/// what equality should mean. Use the predicates below to ask what happened.
#[derive(Debug, Clone)]
pub enum ClipResult<T> {
    /// The plane cut the polyhedron, which is now the part behind it.
    Clipped {
        /// The clipped polyhedron.
        polyhedron: Polyhedron<T>,
        /// The face created where the plane cut through.
        new_face: FaceId,
    },
    /// The whole polyhedron is already behind the plane, so nothing changed.
    Unchanged,
    /// The whole polyhedron is in front of the plane, so nothing is left of it.
    Empty,
}

/// Why a polyhedron could not be clipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipError {
    /// The clipped faces did not form a valid polyhedron.
    ///
    /// The cutting plane passes too close to an existing vertex or edge to resolve the cut at the
    /// available precision.
    Degenerate(BuildError),
}

impl std::fmt::Display for ClipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipError::Degenerate(cause) => {
                write!(f, "the clipped faces do not form a polyhedron: {cause}")
            }
        }
    }
}

impl std::error::Error for ClipError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClipError::Degenerate(cause) => Some(cause),
        }
    }
}

impl<T> ClipResult<T> {
    /// Returns whether the plane left the polyhedron untouched.
    #[inline]
    pub fn is_unchanged(&self) -> bool {
        matches!(self, ClipResult::Unchanged)
    }

    /// Returns whether the plane cut the polyhedron away entirely.
    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(self, ClipResult::Empty)
    }

    /// Returns whether the plane cut the polyhedron.
    #[inline]
    pub fn is_clipped(&self) -> bool {
        matches!(self, ClipResult::Clipped { .. })
    }

    /// Returns the clipped polyhedron, or `None` if the plane did not cut it.
    #[inline]
    pub fn polyhedron(&self) -> Option<&Polyhedron<T>> {
        match self {
            ClipResult::Clipped { polyhedron, .. } => Some(polyhedron),
            _ => None,
        }
    }

    /// Consumes this result and returns the clipped polyhedron, if there is one.
    #[inline]
    pub fn into_polyhedron(self) -> Option<Polyhedron<T>> {
        match self {
            ClipResult::Clipped { polyhedron, .. } => Some(polyhedron),
            _ => None,
        }
    }
}

impl<T> FromPlanes<T> {
    /// Returns the built polyhedron, or `None` if the planes enclose nothing.
    #[inline]
    pub fn polyhedron(&self) -> Option<&Polyhedron<T>> {
        match self {
            FromPlanes::Built { polyhedron, .. } => Some(polyhedron),
            FromPlanes::Empty { .. } => None,
        }
    }

    /// Consumes this result and returns the built polyhedron, if there is one.
    #[inline]
    pub fn into_polyhedron(self) -> Option<Polyhedron<T>> {
        match self {
            FromPlanes::Built { polyhedron, .. } => Some(polyhedron),
            FromPlanes::Empty { .. } => None,
        }
    }
}

impl<T: Float> Polyhedron<T> {
    /// Returns where this polyhedron sits relative to the given plane, if it is entirely
    /// on one side.
    ///
    /// Returns `None` when the plane actually cuts through it.
    fn side_of(&self, plane: &Plane<T, 3>, epsilon: T) -> Option<ClipResult<T>> {
        let mut above = 0usize;
        let mut below = 0usize;

        for vertex in &self.vertices {
            match plane.point_status(vertex.position, epsilon) {
                PlaneStatus::Above => above += 1,
                PlaneStatus::Below => below += 1,
                // Vertices exactly on the plane belong to whichever side survives.
                PlaneStatus::Inside => {}
            }
        }

        if above == 0 {
            Some(ClipResult::Unchanged)
        } else if below == 0 {
            Some(ClipResult::Empty)
        } else {
            None
        }
    }

    /// Clips this polyhedron with the given plane, keeping the part behind it.
    ///
    /// Vertices within `epsilon` of the plane count as being on it.
    pub fn clip(
        &self,
        plane: &Plane<T, 3>,
        epsilon: T,
    ) -> Result<ClipResult<T>, ClipError> {
        if let Some(trivial) = self.side_of(plane, epsilon) {
            return Ok(trivial);
        }

        // Clip each existing face, keeping the part behind the plane. Where a face is cut,
        // the two points it was cut at become an edge of the new face.
        let mut faces: Vec<Vec<Vector<T, 3>>> = Vec::with_capacity(self.faces.len() + 1);
        let mut cut_points: Vec<Vector<T, 3>> = Vec::new();

        for index in 0..self.faces.len() {
            let positions = self.face_positions(FaceId::from_index(index));
            let clipped = clip_polygon(plane, &positions, epsilon);

            if clipped.len() >= 3 {
                for point in &clipped {
                    if plane.point_status(*point, epsilon) == PlaneStatus::Inside {
                        push_distinct(&mut cut_points, *point, epsilon);
                    }
                }
                faces.push(clipped);
            }
        }

        if cut_points.len() < 3 {
            return Err(ClipError::Degenerate(BuildError::DegenerateFace {
                face: faces.len(),
                vertices: cut_points.len(),
            }));
        }

        // The cut points come off the faces in no particular order, so wind them around
        // the plane's normal to make a face of them. The new face looks along the plane's
        // normal, away from the part we kept.
        wind_around(&mut cut_points, plane);
        faces.push(cut_points);
        let new_face_index = faces.len() - 1;

        let polyhedron = Polyhedron::from_faces(&faces, epsilon).map_err(ClipError::Degenerate)?;
        Ok(ClipResult::Clipped {
            polyhedron,
            new_face: FaceId::from_index(new_face_index),
        })
    }

    /// Builds a polyhedron by clipping the given box with each of the given planes.
    ///
    /// This is how a brush is built from its faces. Planes that do not touch the shape are
    /// reported rather than silently ignored, so a caller loading a map file can tell the
    /// difference between a redundant plane and a broken brush.
    pub fn from_planes(
        bounds: &geometry_math::bbox::Bbox<T, 3>,
        planes: &[Plane<T, 3>],
        epsilon: T,
    ) -> Result<FromPlanes<T>, ClipError> {
        // The same operation as clipping a `Polyhedron` by each plane in turn, but the
        // intermediate half-edge mesh is never built: the shape is carried as a soup of
        // face polygons and the `Polyhedron` is assembled once, at the end. Building it
        // per plane was most of `build_tree`'s cost, since each build re-derives every
        // vertex, edge and half-edge.
        let mut faces: Vec<Vec<Vector<T, 3>>> = bounds
            .faces()
            .iter()
            .map(|face| face.vertices.to_vec())
            .collect();
        let mut redundant = Vec::new();
        // The clip passes' working buffer, traded through every cut below.
        let mut scratch: Vec<Vector<T, 3>> = Vec::new();

        for (index, plane) in planes.iter().enumerate() {
            // The trivial cases first: the plane entirely behind every vertex cuts
            // nothing, entirely in front of every vertex leaves nothing.
            let (mut any_above, mut any_below) = (false, false);
            for polygon in &faces {
                for point in polygon {
                    match plane.point_status(*point, epsilon) {
                        PlaneStatus::Above => any_above = true,
                        PlaneStatus::Below => any_below = true,
                        PlaneStatus::Inside => {}
                    }
                }
            }
            if !any_above {
                redundant.push(index);
                continue;
            }
            if !any_below {
                return Ok(FromPlanes::Empty { plane: index });
            }

            // Clip each face, keeping the part behind the plane, and collect the points
            // where the cut crosses, which wind into the new face. The polygons are
            // moved rather than copied -- a face the plane does not touch keeps its
            // buffer, and a face that is cut trades buffers with `scratch` -- so the
            // pass allocates nothing per polygon. This loop is where `build_tree`
            // spends its time, one cut per face per plane per brush.
            let mut next: Vec<Vec<Vector<T, 3>>> = Vec::with_capacity(faces.len() + 1);
            let mut cut_points: Vec<Vector<T, 3>> = Vec::new();
            for mut polygon in std::mem::take(&mut faces) {
                clip_polygon_in_place(plane, &mut polygon, &mut scratch, epsilon);
                if polygon.len() >= 3 {
                    for point in &polygon {
                        if plane.point_status(*point, epsilon) == PlaneStatus::Inside {
                            push_distinct(&mut cut_points, *point, epsilon);
                        }
                    }
                    next.push(polygon);
                }
            }
            if cut_points.len() < 3 {
                return Err(ClipError::Degenerate(BuildError::DegenerateFace {
                    face: next.len(),
                    vertices: cut_points.len(),
                }));
            }
            wind_around(&mut cut_points, plane);
            next.push(cut_points);
            faces = next;
        }

        let polyhedron =
            Polyhedron::from_faces(&faces, epsilon).map_err(ClipError::Degenerate)?;
        Ok(FromPlanes::Built {
            polyhedron,
            redundant,
        })
    }
}

/// The outcome of building a polyhedron from a set of planes.
#[derive(Debug, Clone)]
pub enum FromPlanes<T> {
    /// The planes enclose a volume.
    Built {
        /// The resulting polyhedron.
        polyhedron: Polyhedron<T>,
        /// The indices of planes that cut nothing off, and so contributed no face.
        redundant: Vec<usize>,
    },
    /// The planes enclose nothing.
    Empty {
        /// The index of the plane that emptied the shape.
        plane: usize,
    },
}

/// Clips a convex polygon by a plane, keeping the part behind it.
///
/// [`geometry_math::intersection::polygon_clip_by_plane`] returns nothing when the plane does not
/// cut the polygon, which loses the distinction between "entirely in front" and "entirely
/// behind"; that distinction matters here, so the two cases are handled first.
fn clip_polygon<T: Float>(
    plane: &Plane<T, 3>,
    positions: &[Vector<T, 3>],
    epsilon: T,
) -> Vec<Vector<T, 3>> {
    let any_above = positions
        .iter()
        .any(|p| plane.point_status(*p, epsilon) == PlaneStatus::Above);

    if !any_above {
        return positions.to_vec();
    }
    intersection::polygon_clip_by_plane(plane, positions)
}

/// [`clip_polygon`] without the copies: a polygon the plane does not touch is left
/// exactly as it is, and one that is cut trades buffers with `scratch`.
fn clip_polygon_in_place<T: Float>(
    plane: &Plane<T, 3>,
    polygon: &mut Vec<Vector<T, 3>>,
    scratch: &mut Vec<Vector<T, 3>>,
    epsilon: T,
) {
    let any_above = polygon
        .iter()
        .any(|p| plane.point_status(*p, epsilon) == PlaneStatus::Above);

    if !any_above {
        return;
    }
    intersection::polygon_clip_by_plane_into(plane, polygon, scratch);
    std::mem::swap(polygon, scratch);
}

/// Adds `point` to `points` unless one within `epsilon` of it is already there.
fn push_distinct<T: Float>(points: &mut Vec<Vector<T, 3>>, point: Vector<T, 3>, epsilon: T) {
    let already_there = points
        .iter()
        .any(|p| vec::squared_distance(*p, point) <= epsilon * epsilon);
    if !already_there {
        points.push(point);
    }
}

/// Sorts the given coplanar points into a convex ring wound clockwise as seen from
/// outside, i.e. from the direction the plane's normal points.
///
/// The points come from clipping individual faces, so they arrive in whatever order the
/// faces happened to be in. Sorting them by their angle about their own centre puts them
/// back in order, which works because the cut of a convex solid is a convex polygon.
fn wind_around<T: Float>(points: &mut [Vector<T, 3>], plane: &Plane<T, 3>) {
    let center = vec::average(points.iter().copied()).expect("at least three cut points");

    // Two perpendicular directions in the plane, forming the frame the angle is measured
    // in. Crossing u with the normal rather than the normal with u makes increasing angle
    // run clockwise as seen from outside, which is the winding the mesh expects.
    let u = vec::normalize(points[0] - center);
    let v = vec::cross(u, plane.normal);

    // The diamond angle: the position of the direction along the edge of the unit
    // diamond, shifted to grow over (-pi, pi] exactly as `atan2` does -- the same order,
    // none of the trigonometry, which is what the profile had under this sort.
    let key = |p: Vector<T, 3>| {
        let d = p - center;
        let x = vec::dot(d, u);
        let y = vec::dot(d, v);
        let denominator = geometry_math::scalar::abs(x) + geometry_math::scalar::abs(y);
        if denominator <= T::ZERO {
            return T::ZERO;
        }
        let q = x / denominator;
        if y > T::ZERO {
            T::ONE - q
        } else if y < T::ZERO {
            q - T::ONE
        } else if x >= T::ZERO {
            T::ZERO
        } else {
            T::ONE + T::ONE
        }
    };
    points.sort_by(|a, b| {
        key(*a)
            .partial_cmp(&key(*b))
            .expect("cut point angles are finite")
    });
}
