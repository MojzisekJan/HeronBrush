//! A brush: the convex solid a level is built from.
//!
//! A brush is defined by its faces' planes, not by its vertices — that is what the `.map`
//! format stores, and it is what keeps a brush convex by construction. The geometry is
//! derived: the world box is clipped by each plane in turn, and the resulting
//! [`Polyhedron`] is matched back to the faces that produced it.

use geometry_math::bbox::Bbox3d;
use geometry_math::mat::Mat4x4d;
use geometry_math::mat_ext;
use geometry_math::plane::Plane3d;
use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vec2f, Vec3d};

use crate::brush_face::BrushFace;
use crate::polyhedron::{ClipError, FaceId, FromPlanes, HullError, Polyhedron};
use crate::uv::{UvAttributes, UvCoordSystem};

/// A convex solid, with a material on each of its faces.
#[derive(Debug, Clone)]
pub struct Brush {
    faces: Vec<BrushFace>,
    geometry: Polyhedron<f64>,
    /// For each face of [`Brush::geometry`], which of [`Brush::faces`] it came from.
    face_of_geometry: Vec<usize>,
}

/// The size a face's material is taken to be when there is none loaded.
///
/// `BrushFace::textureSize` returns this too. It only decides how far an offset may drift
/// before it wraps, so with no material there is nothing to wrap against.
const NO_MATERIAL_SIZE: Vec2f = Vec2f::new(1.0, 1.0);

/// Why a brush could not be built.
#[derive(Debug, Clone, PartialEq)]
pub enum BrushError {
    /// Fewer than four faces were given, so they cannot enclose a volume.
    NotEnoughFaces {
        /// How many faces were given.
        given: usize,
    },
    /// The planes enclose no volume at all.
    Empty {
        /// The index of the face whose plane emptied the brush.
        face: usize,
    },
    /// The brush has no face with the given index.
    NoSuchFace {
        /// The index that was asked for.
        index: usize,
    },
    /// The geometry could not be built.
    Geometry(ClipError),
    /// A face of the finished geometry matched no input face.
    ///
    /// This should not happen for well-formed input; it means the geometry drifted far
    /// enough from the planes that produced it that the two can no longer be matched.
    UnmatchedGeometryFace {
        /// The unmatched face's plane.
        plane: Plane3d,
    },
}

impl std::fmt::Display for BrushError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrushError::NotEnoughFaces { given } => write!(
                f,
                "a brush needs at least 4 faces to enclose a volume, got {given}"
            ),
            BrushError::Empty { face } => {
                write!(f, "the planes enclose no volume; face {face} empties the brush")
            }
            BrushError::NoSuchFace { index } => {
                write!(f, "the brush has no face {index}")
            }
            BrushError::Geometry(cause) => write!(f, "{cause}"),
            BrushError::UnmatchedGeometryFace { plane } => write!(
                f,
                "a face of the built geometry at distance {} does not match any of the brush's own faces",
                plane.distance
            ),
        }
    }
}

impl std::error::Error for BrushError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BrushError::Geometry(cause) => Some(cause),
            _ => None,
        }
    }
}

/// What cutting a brush with a plane did to it.
#[derive(Debug, Clone)]
pub enum ClipOutcome {
    /// The brush is entirely behind the plane, so nothing changed.
    Unchanged,
    /// The brush is entirely in front of the plane, so nothing is left of it.
    Empty,
    /// The plane cut the brush.
    Clipped(Brush),
}

impl ClipOutcome {
    /// Returns the brush that is left, or `None` if the cut removed all of it.
    ///
    /// An unchanged brush is not returned here, since the caller already has it.
    pub fn clipped(self) -> Option<Brush> {
        match self {
            ClipOutcome::Clipped(brush) => Some(brush),
            _ => None,
        }
    }
}

/// Why a brush's vertices could not be moved.
#[derive(Debug, Clone, PartialEq)]
pub enum VertexError {
    /// None of the given positions is a vertex of this brush.
    NoVerticesMatched,
    /// No face of this brush passes through the given point.
    NoFaceThere,
    /// The moved vertices no longer enclose a volume.
    Collapsed(HullError),
}

impl std::fmt::Display for VertexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VertexError::NoVerticesMatched => {
                f.write_str("none of the given positions is a vertex of this brush")
            }
            VertexError::NoFaceThere => {
                f.write_str("no face of this brush passes through that point")
            }
            VertexError::Collapsed(cause) => {
                write!(f, "moving those vertices would collapse the brush: {cause}")
            }
        }
    }
}

impl std::error::Error for VertexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VertexError::Collapsed(cause) => Some(cause),
            _ => None,
        }
    }
}

/// Why a brush's vertices could not be snapped.
#[derive(Debug, Clone, PartialEq)]
pub enum SnapError {
    /// The snapped vertices no longer enclose a volume.
    ///
    /// This is what happens when the grid is coarse enough that two corners of the brush
    /// land on the same grid point, or a whole face collapses to a line. The brush is left
    /// alone rather than turned into something degenerate.
    Collapsed(HullError),
    /// The grid size is not positive.
    InvalidGridSize {
        /// The size that was given.
        given: f64,
    },
}

impl std::fmt::Display for SnapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapError::Collapsed(cause) => {
                write!(f, "snapping to this grid would collapse the brush: {cause}")
            }
            SnapError::InvalidGridSize { given } => {
                write!(f, "the grid size must be positive, got {given}")
            }
        }
    }
}

impl std::error::Error for SnapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SnapError::Collapsed(cause) => Some(cause),
            SnapError::InvalidGridSize { .. } => None,
        }
    }
}

/// How closely a geometry face's plane must match a brush face's to be considered the
/// same face.
///
/// The geometry is built by clipping, which accumulates rounding error, so the match is
/// approximate. The tolerance is loose enough to absorb that drift and tight enough that
/// two genuinely different faces of a brush cannot be confused.
const PLANE_MATCH_NORMAL_TOLERANCE: f64 = 0.001;
/// The distance tolerance used alongside [`PLANE_MATCH_NORMAL_TOLERANCE`].
const PLANE_MATCH_DISTANCE_TOLERANCE: f64 = 0.01;

impl Brush {
    /// Builds a brush from its faces.
    ///
    /// The geometry is the part of `world_bounds` behind every face's plane. Faces whose
    /// plane cuts nothing off are dropped, as they contribute no surface; that is a normal
    /// thing to find in a map file rather than an error.
    pub fn from_faces(world_bounds: &Bbox3d, faces: Vec<BrushFace>) -> Result<Self, BrushError> {
        if faces.len() < 4 {
            return Err(BrushError::NotEnoughFaces { given: faces.len() });
        }

        let planes: Vec<Plane3d> = faces.iter().map(|face| face.plane).collect();
        // Saved snapped faces use real lattice vertices. Reconstruct their hull
        // directly when it has exactly these supporting planes. This avoids clipping
        // tiny near-coplanar facets against the large world box. Arbitrary plane
        // defining points still use the usual intersection path.
        let points: Vec<_> = faces.iter().flat_map(|f| f.points).collect();
        let lattice = points
            .iter()
            .all(|p| (0..3).all(|a| (p[a] * 8.0 - (p[a] * 8.0).round()).abs() < 1e-8));
        let direct = if lattice
            && points.iter().all(|p| {
                world_bounds.contains_point(*p)
                    && planes
                        .iter()
                        .all(|plane| vec::dot(plane.normal, *p) - plane.distance <= 1e-7)
            }) {
            Polyhedron::from_points(&points, 1e-7).ok().filter(|hull| {
                hull.faces().iter().all(|f| {
                    planes.iter().any(|p| {
                        vec::length(p.normal - f.plane.normal) < 1e-7
                            && (p.distance - f.plane.distance).abs() < 1e-6
                    })
                })
            })
        } else {
            None
        };
        let geometry = if let Some(hull) = direct {
            hull
        } else {
            match Polyhedron::from_planes(world_bounds, &planes, f64::ALMOST_ZERO)
                .map_err(BrushError::Geometry)?
            {
                FromPlanes::Built { polyhedron, .. } => polyhedron,
                FromPlanes::Empty { plane } => return Err(BrushError::Empty { face: plane }),
            }
        };

        // Match each face of the built geometry back to the brush face whose plane
        // produced it. Matching by plane rather than by clip order is what makes this
        // robust to the geometry builder reordering or dropping faces.
        let mut face_of_geometry = Vec::with_capacity(geometry.face_count());
        for geometry_face in geometry.faces() {
            let matched = faces
                .iter()
                .enumerate()
                .filter(|(_, face)| planes_match(&face.plane, &geometry_face.plane))
                // Nearby facets can both pass the tolerance. The first match can
                // assign two polygons to one source and drop the other at export.
                .min_by(|(_, a), (_, b)| {
                    let error = |face: &BrushFace| {
                        let delta = face.plane.normal - geometry_face.plane.normal;
                        vec::dot(delta, delta)
                            + (face.plane.distance - geometry_face.plane.distance).powi(2)
                    };
                    error(a).total_cmp(&error(b))
                })
                .map(|(index, _)| index)
                .ok_or(BrushError::UnmatchedGeometryFace {
                    plane: geometry_face.plane,
                })?;
            face_of_geometry.push(matched);
        }

        Ok(Self {
            faces,
            geometry,
            face_of_geometry,
        })
    }

    /// Returns the brush's faces, as they were given.
    ///
    /// Some may contribute no surface to the geometry; use
    /// [`Brush::geometry_faces`] to walk only the ones that do.
    #[inline]
    pub fn faces(&self) -> &[BrushFace] {
        &self.faces
    }

    /// Returns the brush's geometry.
    #[inline]
    pub fn geometry(&self) -> &Polyhedron<f64> {
        &self.geometry
    }

    /// Returns the smallest axis aligned box containing the brush.
    #[inline]
    pub fn bounds(&self) -> Bbox3d {
        self.geometry.bounds()
    }

    /// Returns which of [`Brush::faces`] the given geometry face came from.
    ///
    /// A pick names a geometry face, but the material lives on the brush's own face, and
    /// the two are not numbered the same: a face whose plane cuts nothing off contributes
    /// no geometry.
    #[inline]
    pub fn source_face_index(&self, geometry_face: usize) -> Option<usize> {
        self.face_of_geometry.get(geometry_face).copied()
    }

    /// Returns each surface of the brush: its face, and the polygon it covers.
    ///
    /// This is what a renderer walks. Faces that cut nothing off do not appear.
    /// [`for_each_geometry_face`](Self::for_each_geometry_face) does the same without a
    /// `Vec` per face, for the paths that only read the polygon.
    pub fn geometry_faces(&self) -> impl Iterator<Item = (&BrushFace, Vec<Vec3d>)> + '_ {
        (0..self.geometry.face_count()).map(move |index| {
            let face_id = FaceId(index as u32);
            let face = &self.faces[self.face_of_geometry[index]];
            (face, self.geometry.face_positions(face_id))
        })
    }

    /// Calls `visit` with each surface's geometry-face index, its [`BrushFace`] and the
    /// polygon it covers, reusing one buffer for the polygon across the whole walk.
    pub fn for_each_geometry_face(&self, mut visit: impl FnMut(usize, &BrushFace, &[Vec3d])) {
        let mut polygon: Vec<Vec3d> = Vec::new();
        for index in 0..self.geometry.face_count() {
            self.geometry
                .face_positions_into(FaceId(index as u32), &mut polygon);
            visit(index, &self.faces[self.face_of_geometry[index]], &polygon);
        }
    }

    /// Returns whether the given point is inside the brush.
    #[inline]
    pub fn contains_point(&self, point: Vec3d) -> bool {
        self.geometry.contains_point(point, f64::ALMOST_ZERO)
    }

    /// Returns the brush's vertex positions.
    /// How much space the brush encloses.
    ///
    /// The divergence theorem over the faces: each face contributes a sixth of the
    /// determinant of the tetrahedra it makes with the origin. Exact for any convex
    /// solid, and it does not care where the origin is.
    ///
    /// What it is for: deciding whether two brushes can be joined into one without the
    /// join swallowing space that was not theirs. The convex hull of two brushes has the
    /// volume of both only when their union was already convex.
    pub fn volume(&self) -> f64 {
        let geometry = self.geometry();
        let mut total = 0.0;
        for index in 0..geometry.face_count() {
            let corners = geometry.face_positions(crate::polyhedron::FaceId(index as u32));
            for at in 1..corners.len().saturating_sub(1) {
                let (a, b, c) = (corners[0], corners[at], corners[at + 1]);
                total += vec::dot(a, vec::cross(b, c));
            }
        }
        (total / 6.0).abs()
    }

    /// Returns the brush's vertex positions.
    pub fn vertex_positions(&self) -> Vec<Vec3d> {
        self.geometry
            .vertices()
            .iter()
            .map(|vertex| vertex.position)
            .collect()
    }

    /// Builds a brush from a polyhedron, putting the same material on every face.
    ///
    /// This is how a brush is made from geometry rather than from planes — a shape
    /// generator, or an edit that moved vertices.
    pub fn from_polyhedron(geometry: Polyhedron<f64>, material_name: &str) -> Self {
        let mut faces = Vec::with_capacity(geometry.face_count());
        for index in 0..geometry.face_count() {
            let face_id = FaceId(index as u32);
            let positions = geometry.face_positions(face_id);
            faces.push(BrushFace {
                points: [positions[0], positions[1], positions[2]],
                plane: geometry.face_plane(face_id),
                material_name: material_name.into(),
                uv_coord_system: crate::uv::UvCoordSystem::Paraxial,
                uv_attributes: crate::uv::UvAttributes::default(),
                surface_attributes: None,
                color: None,
                surface_material: None,
                export_hidden: false,
                primitive_uv: None,
                line: 0,
            });
        }

        let face_of_geometry = (0..geometry.face_count()).collect();
        Self {
            faces,
            geometry,
            face_of_geometry,
        }
    }

    /// Returns this brush transformed by the given affine matrix.
    ///
    /// An affine transform maps a convex solid to a convex solid, so the result is still a
    /// brush. A transform that mirrors reverses which way round a face's points run, so
    /// the winding is put back; otherwise every face would end up with its normal pointing
    /// into the brush.
    pub fn transform(
        &self,
        world_bounds: &Bbox3d,
        transform: &Mat4x4d,
    ) -> Result<Self, BrushError> {
        self.transform_with_alignment(world_bounds, transform, false, |_| NO_MATERIAL_SIZE)
    }

    /// Returns this brush transformed, keeping the material where it is on each face when
    /// asked to.
    ///
    /// `Brush::transform` with `lockAlignment`, which is what `Preferences::AlignmentLock`
    /// decides. With the lock the material stays put on the face as the face moves; with
    /// it off the material stays put in the world and the face slides across it.
    ///
    /// The size of each face's material is asked for by name, since how far an offset may
    /// drift before it wraps depends on it and only the editor knows what is loaded.
    pub fn transform_with_alignment(
        &self,
        world_bounds: &Bbox3d,
        transform: &Mat4x4d,
        lock_alignment: bool,
        material_size: impl Fn(&str) -> Vec2f,
    ) -> Result<Self, BrushError> {
        let preserves_orientation = mat_ext::is_orientation_preserving_transform(transform);
        let shift = mat_ext::translation_of(transform);

        let faces: Vec<BrushFace> = self
            .faces
            .iter()
            .enumerate()
            .map(|(index, face)| {
                let mut points = [
                    *transform * face.points[0],
                    *transform * face.points[1],
                    *transform * face.points[2],
                ];
                if !preserves_orientation {
                    points.swap(1, 2);
                }

                let mut moved = face.clone();
                moved.points = points;
                moved.plane = geometry_math::plane::from_points(points[0], points[1], points[2])
                    .unwrap_or(face.plane);

                // The invariant is the point on the face whose UV must not move, and
                // working it out walks the face's geometry. Two ways out of that. With
                // the alignment lock off nothing reads it. And under a translation the
                // whole face moves rigidly, so holding any point of it still holds all
                // of them -- the nearest point of its plane to the origin will do, and
                // costs a multiply where the centre of the face costs a walk.
                let invariant = match (lock_alignment, shift.is_some()) {
                    (false, _) => Vec3d::zero(),
                    (true, true) => face.plane.normal * face.plane.distance,
                    (true, false) => self.face_invariant(index),
                };
                let (system, attributes) = face.uv_coord_system.transformed(
                    &face.uv_attributes,
                    &face.plane,
                    &moved.plane,
                    transform,
                    material_size(&face.material_name),
                    lock_alignment,
                    invariant,
                );
                moved.uv_coord_system = system;
                moved.uv_attributes = attributes;
                moved
            })
            .collect();

        // The geometry the brush already has can be carried through the transformation
        // instead of being worked out again from the planes, whenever the transformation
        // keeps the arrangement of corners and edges and faces -- which every invertible
        // one that does not mirror does. That rebuild is what a drag of a large selection
        // spends all of its time on: for the 962 brushes of the monkey, 116ms a step
        // against under one, whether it is a move, a turn or a scale.
        //
        // A mirror is not among them: it turns the winding inside out, and the solid
        // that comes back has its faces the other way round. That goes the long way.
        if preserves_orientation {
            let mut moved = self.clone();
            match shift {
                Some(by) => moved.geometry.translate(by),
                None => moved
                    .geometry
                    .transform(transform, |at| faces[moved.face_of_geometry[at]].plane),
            }
            // Only while it stays inside the world. A brush pushed past the edge is
            // clipped by it, and clipping is the long way round's job.
            if world_bounds.contains(&moved.geometry.bounds()) {
                moved.faces = faces;
                return Ok(moved);
            }
        }

        Self::from_faces(world_bounds, faces)
    }

    /// The point on a face that must keep its UV coordinates when the brush moves.
    ///
    /// `BrushFace::transform` uses the face's centre when there is geometry to take one
    /// from, and a point on its plane otherwise.
    fn face_invariant(&self, index: usize) -> Vec3d {
        match self.face_polygon(index) {
            Some(polygon) if !polygon.is_empty() => {
                polygon
                    .iter()
                    .fold(Vec3d::zero(), |sum, point| sum + *point)
                    / polygon.len() as f64
            }
            _ => {
                let plane = self.faces[index].plane;
                plane.normal * plane.distance
            }
        }
    }

    /// Returns this brush moved by the given delta.
    ///
    /// Translation is exact: the planes move with their faces and the geometry is rebuilt
    /// from them, so a brush moved by a whole number of units and moved back is where it
    /// started.
    pub fn translate(&self, world_bounds: &Bbox3d, delta: Vec3d) -> Result<Self, BrushError> {
        self.translate_with_alignment(world_bounds, delta, false, |_| NO_MATERIAL_SIZE)
    }

    /// Returns this brush moved, keeping the material where it is on each face when asked
    /// to.
    ///
    /// Moving is kept apart from the general transform because it is exact: the planes
    /// move with their faces rather than being rebuilt from transformed points, so a
    /// brush moved by a whole number of units and moved back is where it started.
    pub fn translate_with_alignment(
        &self,
        world_bounds: &Bbox3d,
        delta: Vec3d,
        lock_alignment: bool,
        material_size: impl Fn(&str) -> Vec2f,
    ) -> Result<Self, BrushError> {
        let transform = mat_ext::translation_matrix(delta);
        let faces = self
            .faces
            .iter()
            .map(|face| {
                let mut moved = face.clone();
                moved.points = [
                    face.points[0] + delta,
                    face.points[1] + delta,
                    face.points[2] + delta,
                ];
                // Moving a plane along its own normal is the component of the delta that
                // changes its distance from the origin; the normal itself does not turn.
                moved.plane.distance = face.plane.distance + vec::dot(delta, face.plane.normal);

                // The invariant is the point on the face whose texture must not slide.
                // The face moves rigidly, so holding any of its points holds all of
                // them, and the nearest point of its plane to the origin costs a
                // multiply where the centre of the face costs a walk of its geometry.
                let invariant = if lock_alignment {
                    face.plane.normal * face.plane.distance
                } else {
                    Vec3d::zero()
                };
                let (system, attributes) = face.uv_coord_system.transformed(
                    &face.uv_attributes,
                    &face.plane,
                    &moved.plane,
                    &transform,
                    material_size(&face.material_name),
                    lock_alignment,
                    invariant,
                );
                moved.uv_coord_system = system;
                moved.uv_attributes = attributes;
                moved
            })
            .collect();

        // The solid moves with its faces. Working it out again from the planes instead
        // is slower and, for a thin brush a long way from the origin, wrong: three
        // points defining a plane lose their relative precision as the coordinates grow,
        // and a wedge four units thick collapses to nothing. That is what refused a
        // 256-unit move of the monkey while a 128-unit one went through, and a refused
        // move is a drag that stops following the pointer and does not start again until
        // the pointer comes back within range.
        let mut moved = self.clone();
        moved.geometry.translate(delta);
        if world_bounds.contains(&moved.geometry.bounds()) {
            moved.faces = faces;
            return Ok(moved);
        }

        // Past the edge of the world it has to be clipped, which is what the long way
        // round is for.
        Self::from_faces(world_bounds, faces)
    }

    /// Returns this brush cut by the given plane, keeping the part behind it.
    ///
    /// The face the cut creates takes its material from whichever existing face points
    /// most nearly the same way, which is what a clip should look like: the new surface
    /// matches what it was cut out of.
    pub fn clip(&self, plane: &Plane3d) -> ClipOutcome {
        match self.geometry.clip(plane, f64::ALMOST_ZERO) {
            Ok(crate::polyhedron::ClipResult::Unchanged) => ClipOutcome::Unchanged,
            Ok(crate::polyhedron::ClipResult::Empty) => ClipOutcome::Empty,
            Ok(crate::polyhedron::ClipResult::Clipped { polyhedron, .. }) => {
                ClipOutcome::Clipped(self.rebuilt_with(polyhedron))
            }
            // A cut too close to an existing vertex or edge to resolve leaves the brush
            // alone, which is what the caller wants: nothing happened.
            Err(_) => ClipOutcome::Unchanged,
        }
    }

    /// Returns this brush's face planes.
    pub fn face_planes(&self) -> Vec<Plane3d> {
        (0..self.geometry.face_count())
            .map(|index| self.geometry.face_plane(FaceId(index as u32)))
            .collect()
    }

    /// Returns whether this brush and the given one share any volume.
    pub fn intersects(&self, other: &Self) -> bool {
        // Cheap rejection first: most pairs of brushes in a map are nowhere near
        // each other.
        self.bounds().intersects(&other.bounds()) && crate::csg::intersect(self, other).is_some()
    }

    /// Returns whether every vertex of the given brush lies inside this one.
    ///
    /// `Brush::contains(const Brush&)`, which is how "Select Inside" decides a brush is
    /// wholly within the selection.
    pub fn contains_brush(&self, other: &Self) -> bool {
        other
            .vertex_positions()
            .iter()
            .all(|&vertex| self.contains_point(vertex))
    }

    /// Returns whether every corner of the given box lies inside this brush.
    ///
    /// The box stands in for a patch or a point entity, which "Select Inside" tests by
    /// its bounds the same way, `Brush::contains(const bbox3d&)`.
    pub fn contains_bbox(&self, bbox: &Bbox3d) -> bool {
        bbox.vertices()
            .iter()
            .all(|&corner| self.contains_point(corner))
    }

    /// Returns whether this brush and the given box share any volume.
    ///
    /// The box overlaps the brush's own bounds and then either a corner of the box is
    /// inside the brush or a vertex of the brush is inside the box. A box that only
    /// clips an edge of the brush without either condition holding is missed, which is
    /// the same corner case `Brush::intersects(const bbox3d&)` leaves to the caller.
    pub fn intersects_bbox(&self, bbox: &Bbox3d) -> bool {
        if !self.bounds().intersects(bbox) {
            return false;
        }
        bbox.vertices()
            .iter()
            .any(|&corner| self.contains_point(corner))
            || self
                .vertex_positions()
                .iter()
                .any(|&vertex| bbox.contains_point(vertex))
    }

    /// Returns this brush with the given vertices moved by the given delta.
    ///
    /// This is what dragging a vertex, an edge or a face all reduce to: an edge is its two
    /// endpoints and a face is its ring of them. The brush is rebuilt as the convex hull
    /// of the result, so vertices that a drag brings together merge and faces that it
    /// flattens away disappear, which is what makes the tool usable for shaping rather
    /// than only for nudging.
    ///
    /// Positions are matched to the brush's own vertices within `epsilon`; any that match
    /// nothing are ignored, since a stale selection should not stop the rest of a drag.
    pub fn move_vertices(
        &self,
        positions: &[Vec3d],
        delta: Vec3d,
        epsilon: f64,
    ) -> Result<Self, VertexError> {
        self.move_vertices_with_uv_lock(positions, delta, epsilon, false, |_| NO_MATERIAL_SIZE)
    }

    /// Returns this brush with the given corners moved, keeping the material where it is
    /// on each face when asked to.
    ///
    /// `Brush::transformVertices` with `uvLock`, which is what `Preferences::UvLock`
    /// decides. Dragging a corner stretches the faces it belongs to; with the lock the
    /// material stretches with them, and without it the material stays as it was and the
    /// face slides across it.
    pub fn move_vertices_with_uv_lock(
        &self,
        positions: &[Vec3d],
        delta: Vec3d,
        epsilon: f64,
        uv_lock: bool,
        material_size: impl Fn(&str) -> Vec2f,
    ) -> Result<Self, VertexError> {
        if delta == Vec3d::zero() {
            return Ok(self.clone());
        }

        let mut moved_any = false;
        let updated: Vec<Vec3d> = self
            .geometry
            .vertices()
            .iter()
            .map(|vertex| {
                let is_moving = positions
                    .iter()
                    .any(|p| vec::squared_distance(*p, vertex.position) <= epsilon * epsilon);
                if is_moving {
                    moved_any = true;
                    vertex.position + delta
                } else {
                    vertex.position
                }
            })
            .collect();

        if !moved_any {
            return Err(VertexError::NoVerticesMatched);
        }

        let geometry =
            Polyhedron::from_points(&updated, f64::ALMOST_ZERO).map_err(VertexError::Collapsed)?;

        let pairs: Vec<(Vec3d, Vec3d)> = if uv_lock {
            self.geometry
                .vertices()
                .iter()
                .zip(&updated)
                .map(|(vertex, to)| (vertex.position, *to))
                .collect()
        } else {
            Vec::new()
        };

        Ok(self.rebuilt_with_uv_lock(geometry, &pairs, material_size))
    }

    /// Returns this brush with one of its faces drawn in and pushed along its normal.
    ///
    /// What "inset and bevel a face" means for a solid made of planes: the corners of that
    /// face move inward across the face by `inset` and along its normal by `depth`, and
    /// the faces around it tilt to meet them. Inset alone leaves a taper, depth alone is
    /// an extrude, and the two together leave a raised panel with sloped sides.
    ///
    /// Refused -- as `Err` -- when the inset eats the face, or when what is left no longer
    /// encloses anything.
    /// Insets the boundary of a selected face region once. Shared selected edges
    /// stay connected; every affected vertex is solved from the same original solid.
    pub fn bevel_face_region(
        &self,
        indices: &[usize],
        inset: f64,
        depth: f64,
    ) -> Result<Self, VertexError> {
        self.bevel_face_region_with_seams(indices, inset, depth, &[])
    }

    /// Coplanar selected faces on other brushes may share these full edges. Their
    /// seam stays in place while the external region boundary is inset.
    pub fn bevel_face_region_with_seams(
        &self,
        indices: &[usize],
        inset: f64,
        depth: f64,
        seams: &[(Vec3d, Vec3d)],
    ) -> Result<Self, VertexError> {
        if !inset.is_finite() || !depth.is_finite() || inset < 0.0 {
            return Err(VertexError::NoFaceThere);
        }
        let mut indices = indices.to_vec();
        indices.sort_unstable();
        indices.dedup();
        if indices.is_empty() {
            return Err(VertexError::NoFaceThere);
        }
        let mut regions = Vec::new();
        for &index in &indices {
            let polygon = self.face_polygon(index).ok_or(VertexError::NoFaceThere)?;
            let face = self.faces.get(index).ok_or(VertexError::NoFaceThere)?;
            let center =
                polygon.iter().copied().fold(Vec3d::zero(), |a, b| a + b) / polygon.len() as f64;
            let mut boundary = Vec::new();
            for (&a, &b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
                let shared = self.faces.iter().enumerate().any(|(other, f)| {
                    other != index
                        && indices.contains(&other)
                        && (vec::dot(f.normal(), a) - f.plane.distance).abs() < 1e-7
                        && (vec::dot(f.normal(), b) - f.plane.distance).abs() < 1e-7
                });
                if shared {
                    continue;
                }
                let mut outward = vec::normalize(vec::cross(b - a, face.normal()));
                if vec::dot(outward, center - a) > 0.0 {
                    outward = -outward;
                }
                let seam = seams.iter().any(|(x, y)| {
                    (vec::squared_distance(a, *x) < 1e-12 && vec::squared_distance(b, *y) < 1e-12)
                        || (vec::squared_distance(a, *y) < 1e-12
                            && vec::squared_distance(b, *x) < 1e-12)
                });
                boundary.push((
                    a,
                    b,
                    Plane3d::from_anchor_and_normal(
                        a - outward * if seam { 0.0 } else { inset },
                        outward,
                    ),
                ));
            }
            regions.push((index, polygon, boundary));
        }
        if depth == 0.0 && inset > 0.0 && regions.iter().all(|(_, _, boundary)| boundary.is_empty())
        {
            return Err(VertexError::NoFaceThere);
        }
        let mut points = Vec::new();
        for vertex in self.geometry.vertices() {
            let position = vertex.position;
            let incident: Vec<_> = regions
                .iter()
                .filter(|(_, polygon, _)| {
                    polygon
                        .iter()
                        .any(|p| vec::squared_distance(*p, position) < 1e-10)
                })
                .collect();
            if incident.is_empty() {
                points.push(position);
                continue;
            }
            let mut constraints = Vec::new();
            for (index, _, boundary) in &incident {
                let mut plane = self.faces[*index].plane;
                plane.distance += depth;
                constraints.push(plane);
                for (a, b, plane) in boundary {
                    if vec::squared_distance(*a, position) < 1e-10
                        || vec::squared_distance(*b, position) < 1e-10
                    {
                        constraints.push(*plane);
                    }
                }
            }
            let mut gram = [[0.0; 3]; 3];
            let mut rhs = Vec3d::zero();
            for plane in &constraints {
                let n = plane.normal;
                rhs += n * (plane.distance - vec::dot(n, position));
                for i in 0..3 {
                    for j in 0..3 {
                        gram[i][j] += n[i] * n[j];
                    }
                }
            }
            let inverse = geometry_math::mat::invert(geometry_math::mat::Mat3x3d::from_rows(gram))
                .ok_or(VertexError::NoFaceThere)?;
            let moved = position + inverse * rhs;
            if constraints
                .iter()
                .any(|p| (vec::dot(p.normal, moved) - p.distance).abs() > 1e-6)
                || incident.iter().any(|(_, _, boundary)| {
                    boundary
                        .iter()
                        .any(|(_, _, p)| vec::dot(p.normal, moved) - p.distance > 1e-6)
                })
            {
                return Err(VertexError::NoFaceThere);
            }
            points.push(moved);
        }
        let geometry =
            Polyhedron::from_points(&points, f64::ALMOST_ZERO).map_err(VertexError::Collapsed)?;
        for (index, _, _) in regions {
            let expected = self.faces[index].plane;
            if !geometry.faces().iter().any(|f| {
                vec::length(f.plane.normal - expected.normal) < 1e-6
                    && (f.plane.distance - expected.distance - depth).abs() < 1e-6
            }) {
                return Err(VertexError::NoFaceThere);
            }
        }
        Ok(self.rebuilt_with_uv_lock(geometry, &[], |_| NO_MATERIAL_SIZE))
    }

    /// Insets one face independently and optionally raises it along its normal.
    pub fn bevel_face(
        &self,
        face_index: usize,
        inset: f64,
        depth: f64,
    ) -> Result<Self, VertexError> {
        let polygon = self
            .face_polygon(face_index)
            .ok_or(VertexError::NoFaceThere)?;
        let normal = self
            .faces
            .get(face_index)
            .ok_or(VertexError::NoFaceThere)?
            .normal();
        let moved = inset_polygon(&polygon, normal, inset).ok_or(VertexError::NoFaceThere)?;

        // Every corner that is not on this face stays where it is; the face's own corners
        // are the drawn-in ones, pushed out along the normal.
        let epsilon = f64::ALMOST_ZERO;
        let mut points: Vec<Vec3d> = self
            .geometry
            .vertices()
            .iter()
            .map(|vertex| vertex.position)
            .filter(|position| {
                !polygon
                    .iter()
                    .any(|corner| vec::squared_distance(*corner, *position) <= epsilon)
            })
            .collect();
        points.extend(moved.iter().map(|corner| *corner + normal * depth));

        let geometry =
            Polyhedron::from_points(&points, f64::ALMOST_ZERO).map_err(VertexError::Collapsed)?;
        Ok(self.rebuilt_with_uv_lock(geometry, &[], |_| NO_MATERIAL_SIZE))
    }

    /// A new brush standing on one of this brush's faces: the face drawn in by `inset`,
    /// raised by `depth`.
    ///
    /// A panel when the depth is positive and the shape of a recess when it is negative,
    /// which is then taken out of the brush it stands on.
    pub fn panel_on_face(
        &self,
        face_index: usize,
        inset: f64,
        depth: f64,
    ) -> Result<Self, VertexError> {
        if depth == 0.0 {
            return Err(VertexError::NoFaceThere);
        }
        let polygon = self
            .face_polygon(face_index)
            .ok_or(VertexError::NoFaceThere)?;
        let normal = self
            .faces
            .get(face_index)
            .ok_or(VertexError::NoFaceThere)?
            .normal();
        let footprint = inset_polygon(&polygon, normal, inset).ok_or(VertexError::NoFaceThere)?;

        let mut points = footprint.clone();
        points.extend(footprint.iter().map(|corner| *corner + normal * depth));

        let geometry =
            Polyhedron::from_points(&points, f64::ALMOST_ZERO).map_err(VertexError::Collapsed)?;
        let material = self
            .faces
            .get(face_index)
            .map(|face| face.material_name.clone())
            .unwrap_or_default();
        Ok(Self::from_polyhedron(geometry, &material))
    }

    /// Returns this brush with one more corner: the given point, moved by the given delta.
    ///
    /// `Brush::addVertex` and the drag that follows it, which are one operation here. A
    /// point on an edge or a face is not a corner of the hull -- the hull is the same
    /// shape with or without it -- so adding one and moving it later would add nothing to
    /// move. Adding it where it is going is what makes the split visible: the middle of a
    /// top edge, pulled upwards, leaves a ridge.
    pub fn split_vertex(&self, at: Vec3d, delta: Vec3d) -> Result<Self, VertexError> {
        self.add_vertex(at + delta)
    }

    /// Returns this brush with one more corner, at the given point.
    ///
    /// The hull of what it had plus the point, so a point inside it or on its surface
    /// changes nothing.
    pub fn add_vertex(&self, position: Vec3d) -> Result<Self, VertexError> {
        let mut points: Vec<Vec3d> = self
            .geometry
            .vertices()
            .iter()
            .map(|vertex| vertex.position)
            .collect();
        points.push(position);

        let geometry =
            Polyhedron::from_points(&points, f64::ALMOST_ZERO).map_err(VertexError::Collapsed)?;
        Ok(self.rebuilt_with_uv_lock(geometry, &[], |_| NO_MATERIAL_SIZE))
    }

    /// Returns this brush without the given vertices.
    ///
    /// `Brush::removeVertices`: what is left is rebuilt as the hull of the vertices that
    /// remain, so taking a corner off a cube leaves the slanted face that closes it. A
    /// brush needs four corners to enclose anything, so removing more than that many is
    /// refused rather than leaving nothing.
    pub fn remove_vertices(&self, positions: &[Vec3d], epsilon: f64) -> Result<Self, VertexError> {
        let mut removed_any = false;
        let left: Vec<Vec3d> = self
            .geometry
            .vertices()
            .iter()
            .filter(|vertex| {
                let going = positions
                    .iter()
                    .any(|p| vec::squared_distance(*p, vertex.position) <= epsilon * epsilon);
                removed_any |= going;
                !going
            })
            .map(|vertex| vertex.position)
            .collect();

        if !removed_any {
            return Err(VertexError::NoVerticesMatched);
        }
        if left.len() < 4 {
            return Err(VertexError::Collapsed(HullError::NotEnoughPoints {
                distinct: left.len(),
            }));
        }

        let geometry =
            Polyhedron::from_points(&left, f64::ALMOST_ZERO).map_err(VertexError::Collapsed)?;
        Ok(self.rebuilt_with_uv_lock(geometry, &[], |_| NO_MATERIAL_SIZE))
    }

    /// Returns the vertices, in order, of the face at the given index.
    ///
    /// The geometry is what has vertices; a brush face is a plane, and which part of the
    /// geometry it bounds is the mapping this looks through.
    pub fn face_polygon(&self, face_index: usize) -> Option<Vec<Vec3d>> {
        let geometry_face = self
            .face_of_geometry
            .iter()
            .position(|source| *source == face_index)?;
        Some(self.geometry.face_positions(FaceId(geometry_face as u32)))
    }

    /// Returns which face of the geometry the face at the given index bounds.
    pub fn geometry_face_of(&self, face_index: usize) -> Option<FaceId> {
        self.face_of_geometry
            .iter()
            .position(|source| *source == face_index)
            .map(|index| FaceId(index as u32))
    }

    /// Returns the index of the face whose vertices are the given polygon.
    ///
    /// `Brush::findFace(const std::vector<geometry_math::polygon3d>&)`: the same face of the same
    /// shape, which is how a drag started on one brush finds the matching face on the
    /// others it is being applied to. The comparison is by set rather than by order,
    /// since two brushes meeting at a face wind it opposite ways.
    pub fn find_face_by_polygon(&self, polygon: &[Vec3d], epsilon: f64) -> Option<usize> {
        (0..self.faces.len()).find(|index| {
            let Some(candidate) = self.face_polygon(*index) else {
                return false;
            };
            candidate.len() == polygon.len()
                && candidate.iter().all(|point| {
                    polygon
                        .iter()
                        .any(|other| vec::squared_distance(*point, *other) <= epsilon * epsilon)
                })
        })
    }

    /// Returns the index of the face pointing the given way.
    ///
    /// `Brush::findFace(const geometry_math::vec3d&)`, which is how a face is found again on a brush
    /// that has just been rebuilt.
    pub fn find_face_by_normal(&self, normal: Vec3d, epsilon: f64) -> Option<usize> {
        self.faces
            .iter()
            .position(|face| vec::squared_distance(face.plane.normal, normal) <= epsilon * epsilon)
    }

    /// Returns this brush with one face's plane moved by the given delta.
    ///
    /// `Brush::moveBoundary`. This is what extruding does, and it is not the same as
    /// moving the face's vertices: the plane slides and the brush is rebuilt from its
    /// planes, so the neighbouring faces stretch to meet it wherever it lands. Moving the
    /// vertices instead would leave the neighbours behind.
    pub fn move_boundary(
        &self,
        world_bounds: &Bbox3d,
        face_index: usize,
        delta: Vec3d,
    ) -> Result<Self, BrushError> {
        self.move_boundaries(world_bounds, &[(face_index, delta)])
    }

    /// Returns this brush with several faces' planes moved at once.
    ///
    /// Moving them one after another would not do: rebuilding the brush after the first
    /// move changes where every other face's vertices are, so the second move would be
    /// measured against a brush that no longer matches the drag. Here every plane is
    /// shifted first and the brush is rebuilt once, which is what dragging a corner of a
    /// brush in a flat view needs -- two walls going out together.
    pub fn move_boundaries(
        &self,
        world_bounds: &Bbox3d,
        moves: &[(usize, Vec3d)],
    ) -> Result<Self, BrushError> {
        let mut faces = self.faces.clone();
        for (face_index, delta) in moves {
            let face = faces
                .get_mut(*face_index)
                .ok_or(BrushError::NoSuchFace { index: *face_index })?;

            for point in &mut face.points {
                *point += *delta;
            }
            face.plane =
                geometry_math::plane::from_points(face.points[0], face.points[1], face.points[2])
                    .ok_or(BrushError::NoSuchFace { index: *face_index })?;
        }

        Self::from_faces(world_bounds, faces)
    }

    /// Returns this brush with every face's plane moved outwards by the given distance.
    ///
    /// `Brush::expand`. A negative distance shrinks it, and shrinking a brush past the
    /// point where its planes still enclose anything is an error rather than an empty
    /// brush.
    pub fn expand(&self, world_bounds: &Bbox3d, delta: f64) -> Result<Self, BrushError> {
        let mut faces = self.faces.clone();
        for face in &mut faces {
            let move_amount = face.plane.normal * delta;
            for point in &mut face.points {
                *point += move_amount;
            }
            face.plane =
                geometry_math::plane::from_points(face.points[0], face.points[1], face.points[2])
                    .ok_or(BrushError::NoSuchFace { index: 0 })?;
        }
        Self::from_faces(world_bounds, faces)
    }

    /// Copies each face's material and mapping from whichever of the given faces best
    /// matches it.
    ///
    /// `Brush::cloneFaceAttributesFrom`. A face made by merging or hollowing has no
    /// material of its own; taking it from the face it stands in for is what stops a
    /// merge coming out in the default material.
    ///
    /// The best match is the largest face lying in the same plane, and failing that the
    /// one whose middle is nearest this face's plane.
    /// Copies all attributes without rebuilding geometry. Plane-defining data must
    /// match exactly, so face indices and cached hull topology remain valid.
    pub fn with_exact_face_attributes(&self, faces: &[BrushFace]) -> Option<Self> {
        if faces.len() != self.faces.len()
            || faces
                .iter()
                .zip(&self.faces)
                .any(|(a, b)| a.plane != b.plane || a.points != b.points)
        {
            return None;
        }
        let mut result = self.clone();
        result.faces.clone_from_slice(faces);
        Some(result)
    }

    /// Transfers attributes from coplanar candidates, with nearest-plane fallback.
    pub fn with_face_attributes_from(&self, candidates: &[BrushFace]) -> Self {
        if candidates.is_empty() {
            return self.clone();
        }

        let mut copy = self.clone();
        for index in 0..copy.faces.len() {
            let plane = copy.faces[index].plane;
            let area = |face: &BrushFace| {
                // The face's own extent stands in for its area: what matters is picking
                // the larger of two coplanar candidates, and a bigger face bounds a
                // bigger box.
                let (a, b, c) = (face.points[0], face.points[1], face.points[2]);
                vec::length(vec::cross(b - a, c - a))
            };

            let coplanar: Vec<&BrushFace> = candidates
                .iter()
                .filter(|face| {
                    geometry_math::plane::is_equal(&face.plane, &plane, f64::ALMOST_ZERO)
                })
                .collect();

            let best = if !coplanar.is_empty() {
                coplanar.into_iter().max_by(|a, b| {
                    area(a)
                        .partial_cmp(&area(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            } else {
                candidates.iter().min_by(|a, b| {
                    let distance = |face: &BrushFace| {
                        let center = (face.points[0] + face.points[1] + face.points[2]) / 3.0;
                        plane.point_distance(center).abs()
                    };
                    distance(a)
                        .partial_cmp(&distance(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            };

            if let Some(best) = best {
                copy.faces[index].material_name = best.material_name.clone();
                copy.faces[index].uv_attributes = best.uv_attributes;
                copy.faces[index].surface_attributes = best.surface_attributes;
                // The axes are carried onto this face's own plane rather than taken as
                // they are, which is `WrapStyle::Projection`.
                copy.faces[index].uv_coord_system = best.uv_coord_system.updated_for_normal(
                    best.plane.normal,
                    plane.normal,
                    crate::uv::WrapStyle::Projection,
                );
            }
        }
        copy
    }

    /// Returns this brush with the given face moved by the given delta.
    ///
    /// The face is named by a point on it — what a click gives — so the caller does not
    /// have to know the brush's own numbering.
    pub fn move_face(
        &self,
        point_on_face: Vec3d,
        delta: Vec3d,
        epsilon: f64,
    ) -> Result<Self, VertexError> {
        let face = self
            .face_at(point_on_face, epsilon)
            .ok_or(VertexError::NoFaceThere)?;
        let positions = self.geometry.face_positions(face);
        self.move_vertices(&positions, delta, epsilon)
    }

    /// Returns the face of this brush that the given point lies on, if any.
    pub fn face_at(&self, point: Vec3d, epsilon: f64) -> Option<FaceId> {
        (0..self.geometry.face_count())
            .map(|index| FaceId(index as u32))
            .find(|face| self.geometry.face_plane(*face).point_distance(point).abs() <= epsilon)
    }

    /// Returns this brush rebuilt around new geometry, carrying its materials across.
    ///
    /// Each new face takes the material of whichever old face points most nearly the same
    /// way, which is what survives the vertices having moved.
    fn rebuilt_with(&self, geometry: Polyhedron<f64>) -> Self {
        self.rebuilt_with_uv_lock(geometry, &[], |_| NO_MATERIAL_SIZE)
    }

    /// Rebuilds the faces around new geometry, carrying each face's material with it and
    /// keeping the material where it is on the face when asked to.
    ///
    /// `Brush::updateFacesFromGeometry` with `uvLock`. `moved` pairs each vertex position
    /// this brush had with where it went; an empty list means no lock, which is what
    /// `Preferences::UvLock` is off.
    fn rebuilt_with_uv_lock(
        &self,
        geometry: Polyhedron<f64>,
        moved: &[(Vec3d, Vec3d)],
        material_size: impl Fn(&str) -> Vec2f,
    ) -> Self {
        let mut faces = Vec::with_capacity(geometry.face_count());
        for index in 0..geometry.face_count() {
            let face_id = FaceId(index as u32);
            let plane = geometry.face_plane(face_id);
            let source_index = self.best_matching_face_index(&plane);
            let source = &self.faces[source_index];
            let positions = geometry.face_positions(face_id);

            // With the lock on, the material follows the face's own corners rather than
            // staying where the source face put it.
            let locked = (!moved.is_empty())
                .then(|| self.uv_under_lock(source_index, moved, &material_size))
                .flatten();
            let (uv_coord_system, uv_attributes) =
                locked.unwrap_or((source.uv_coord_system, source.uv_attributes));

            faces.push(BrushFace {
                points: [positions[0], positions[1], positions[2]],
                plane,
                material_name: source.material_name.clone(),
                uv_coord_system,
                uv_attributes,
                surface_attributes: source.surface_attributes,
                // A rebuilt face keeps the colour painted on the face it came from, the
                // same way it keeps the material.
                color: source.color.clone(),
                surface_material: source.surface_material,
                export_hidden: source.export_hidden,
                // A rebuilt face is real geometry, so any pending primitive matrix has
                // already been resolved or is not this face's concern.
                primitive_uv: None,
                line: source.line,
            });
        }

        let face_of_geometry = (0..geometry.face_count()).collect();
        Self {
            faces,
            geometry,
            face_of_geometry,
        }
    }

    /// Returns this brush with every vertex moved to the nearest grid point.
    ///
    /// A brush is defined by planes, so its corners land wherever those planes happen to
    /// meet — after an edit, or after being built by clipping, that is rarely a round
    /// number. Snapping moves the corners onto the grid and rebuilds the brush around
    /// them, which is what keeps a hand-built level's coordinates tidy.
    ///
    /// The rebuilt brush's faces take their material and UV mapping from whichever of this
    /// brush's faces they came from, matched by orientation. A face that snapping removes
    /// entirely takes its material with it.
    ///
    /// Returns an error and leaves this brush untouched if the snapped corners no longer
    /// enclose a volume.
    pub fn snap_vertices(&self, grid_size: f64) -> Result<Self, SnapError> {
        if grid_size <= 0.0 {
            return Err(SnapError::InvalidGridSize { given: grid_size });
        }

        let snapped: Vec<Vec3d> = self
            .geometry
            .vertices()
            .iter()
            .map(|vertex| geometry_math::vec::round(vertex.position / grid_size) * grid_size)
            .collect();

        let geometry = Polyhedron::from_points(&snapped, 1e-7).map_err(SnapError::Collapsed)?;

        Ok(self.rebuilt_with(geometry))
    }

    /// Returns this brush with each vertex moved where `place` puts it, and the faces
    /// rebuilt around them with their materials, as [`Brush::snap_vertices`] does.
    ///
    /// The result is the convex hull of the moved vertices: one moved inside the others
    /// is dropped. Returns an error and leaves this brush untouched if they no longer
    /// enclose a volume.
    pub fn map_vertices(&self, place: impl Fn(Vec3d) -> Vec3d) -> Result<Self, HullError> {
        let moved: Vec<Vec3d> = self
            .geometry
            .vertices()
            .iter()
            .map(|vertex| place(vertex.position))
            .collect();
        let geometry = Polyhedron::from_points(&moved, 1e-7)?;
        Ok(self.rebuilt_with(geometry))
    }

    /// Returns whether every vertex of this brush already sits on the given grid.
    pub fn vertices_are_on_grid(&self, grid_size: f64) -> bool {
        if grid_size <= 0.0 {
            return false;
        }
        self.geometry.vertices().iter().all(|vertex| {
            let snapped = geometry_math::vec::round(vertex.position / grid_size) * grid_size;
            geometry_math::vec::squared_distance(snapped, vertex.position) <= 1e-10
        })
    }

    /// Returns whether any face is still carrying an unconverted brush-primitive texture
    /// matrix (`brushDef` / `brushDef3`).
    pub fn has_pending_primitive_uv(&self) -> bool {
        self.faces.iter().any(|face| face.primitive_uv.is_some())
    }

    /// Returns this brush with every pending brush-primitive texture matrix turned into
    /// real UV axes, `size_of` giving each material's pixel size. `None` when no face was
    /// waiting -- the usual case -- so the caller skips the copy. A face whose material
    /// `size_of` does not know keeps its matrix for a later try.
    pub fn resolved_primitive_uv(&self, size_of: impl Fn(&str) -> Option<Vec2f>) -> Option<Self> {
        if !self.has_pending_primitive_uv() {
            return None;
        }
        let mut faces = self.faces.clone();
        for face in &mut faces {
            if face.primitive_uv.is_some() {
                if let Some(size) = size_of(&face.material_name) {
                    face.resolve_primitive_uv(size);
                }
            }
        }
        Some(Self {
            faces,
            geometry: self.geometry.clone(),
            face_of_geometry: self.face_of_geometry.clone(),
        })
    }

    /// Returns the index of the face whose plane points most nearly the same way as the
    /// given one.
    ///
    /// Used to carry materials across a rebuild. There is always an answer, because a
    /// brush always has at least four faces.
    fn best_matching_face_index(&self, plane: &Plane3d) -> usize {
        self.faces
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                let da = vec::dot(a.plane.normal, plane.normal);
                let db = vec::dot(b.plane.normal, plane.normal);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
            .expect("a brush always has faces")
    }

    /// The mapping the given face ends up with when its corners are dragged and the UV
    /// lock is on.
    ///
    /// `Brush::findTransformForUvLock` and `applyUvLock`: the transformation that takes
    /// the face's own corners where they went is found, the face is transformed by it
    /// with the alignment locked, and the mapping that comes out is what the rebuilt face
    /// wears. Returns nothing when there is no such transformation, in which case the
    /// face keeps the mapping it had.
    fn uv_under_lock(
        &self,
        source_index: usize,
        moved: &[(Vec3d, Vec3d)],
        material_size: &impl Fn(&str) -> Vec2f,
    ) -> Option<(UvCoordSystem, UvAttributes)> {
        let polygon = self.face_polygon(source_index)?;

        let mut unmoved = Vec::new();
        let mut shifted = Vec::new();
        for corner in polygon {
            let to = moved
                .iter()
                .find(|(from, _)| vec::squared_distance(*from, corner) <= f64::ALMOST_ZERO)
                .map_or(corner, |(_, to)| *to);
            if vec::is_equal(&to, &corner, f64::ALMOST_ZERO) {
                unmoved.push(corner);
            } else {
                shifted.push((corner, to));
            }
        }

        // Three corners staying put pin the face down: picture a square with one corner
        // dragged, and there is no one transformation that holds all four mappings.
        if unmoved.len() >= 3 {
            return None;
        }

        // The ones that stayed put first, since they are the ones the mapping should
        // certainly hold.
        let mut reference: Vec<(Vec3d, Vec3d)> =
            unmoved.into_iter().map(|corner| (corner, corner)).collect();
        reference.extend(shifted);
        if reference.len() < 3 {
            return None;
        }

        let transform = mat_ext::points_transformation_matrix_3(
            reference[0].0,
            reference[1].0,
            reference[2].0,
            reference[0].1,
            reference[1].1,
            reference[2].1,
        )?;
        // A degenerate correspondence gives a matrix with nothing usable in it.
        if !(0..4).all(|column| (0..4).all(|row| transform[column][row].is_finite())) {
            return None;
        }

        let source = &self.faces[source_index];
        let moved_plane = source.plane.transform(&transform);
        Some(source.uv_coord_system.transformed(
            &source.uv_attributes,
            &source.plane,
            &moved_plane,
            &transform,
            material_size(&source.material_name),
            true,
            self.face_invariant(source_index),
        ))
    }

    /// Returns the brush's edges as pairs of endpoints.
    ///
    /// This is what the 2D views draw.
    pub fn edge_positions(&self) -> Vec<(Vec3d, Vec3d)> {
        self.geometry
            .edges()
            .iter()
            .map(|edge| {
                let half_edge = self.geometry.half_edge(edge.first);
                (
                    self.geometry.vertex(half_edge.origin).position,
                    self.geometry
                        .vertex(self.geometry.destination(edge.first))
                        .position,
                )
            })
            .collect()
    }
}

/// Returns whether the two planes describe the same surface, within the matching
/// tolerances.
fn planes_match(a: &Plane3d, b: &Plane3d) -> bool {
    vec::dot(a.normal, b.normal) > 1.0 - PLANE_MATCH_NORMAL_TOLERANCE
        && (a.distance - b.distance).abs() < PLANE_MATCH_DISTANCE_TOLERANCE
}

/// Draws a convex polygon in by the same distance all the way round.
///
/// Each edge is moved `distance` toward the middle of the face and the neighbouring edges
/// are met where their lines cross, which is what makes an inset an inset rather than a
/// scaling: a long edge and a short one come in by the same amount. Nothing comes back if
/// the polygon has eaten itself.
fn inset_polygon(polygon: &[Vec3d], normal: Vec3d, distance: f64) -> Option<Vec<Vec3d>> {
    if polygon.len() < 3 {
        return None;
    }
    if distance == 0.0 {
        return Some(polygon.to_vec());
    }

    let count = polygon.len();
    // Which way round the polygon runs, seen from outside the face: what says whether
    // "inward" is one cross product or the other.
    let mut turning = Vec3d::zero();
    for index in 0..count {
        let a = polygon[index];
        let b = polygon[(index + 1) % count];
        turning += vec::cross(a, b);
    }
    let sign = if vec::dot(turning, normal) < 0.0 {
        -1.0
    } else {
        1.0
    };

    // Each edge, moved inward by the distance: a point on it and the way it runs.
    let mut lines = Vec::with_capacity(count);
    for index in 0..count {
        let a = polygon[index];
        let b = polygon[(index + 1) % count];
        let along = b - a;
        if vec::length(along) < f64::ALMOST_ZERO {
            return None;
        }
        let along = vec::normalize(along);
        let inward = vec::normalize(vec::cross(normal, along)) * sign;
        lines.push((a + inward * distance, along));
    }

    // Every corner is where the two edges that meet there meet after the move.
    let mut drawn_in = Vec::with_capacity(count);
    for index in 0..count {
        let (before_point, before_along) = lines[(index + count - 1) % count];
        let (after_point, after_along) = lines[index];
        let corner = meet(before_point, before_along, after_point, after_along, normal)
            // Two edges in a line have nothing to cross at; the moved point will do.
            .unwrap_or(after_point);
        drawn_in.push(corner);
    }

    // An inset that has eaten the polygon is no inset. Every corner has to be on the
    // inward side of every moved edge: past that the edges have crossed each other, and
    // what comes out is the same shape turned inside out rather than a smaller one --
    // which a signed area does not notice, since the corners come round in the same order.
    for (point, along) in &lines {
        let inward = vec::normalize(vec::cross(normal, *along)) * sign;
        for corner in &drawn_in {
            if vec::dot(*corner - *point, inward) < -f64::ALMOST_ZERO {
                return None;
            }
        }
    }
    Some(drawn_in)
}

/// Where two lines in a plane meet.
fn meet(
    one_point: Vec3d,
    one_along: Vec3d,
    other_point: Vec3d,
    other_along: Vec3d,
    normal: Vec3d,
) -> Option<Vec3d> {
    // In the plane, the crossing is found by how far along the first line the second one
    // reaches -- which is a ratio of two areas, and no crossing when the second is zero.
    let denominator = vec::dot(vec::cross(one_along, other_along), normal);
    if denominator.abs() < 1e-9 {
        return None;
    }
    let between = other_point - one_point;
    let along = vec::dot(vec::cross(between, other_along), normal) / denominator;
    Some(one_point + one_along * along)
}

#[cfg(test)]
mod bevel_tests {
    use super::*;
    use geometry_math::bbox::Bbox3d;

    fn world() -> Bbox3d {
        Bbox3d::from_scalars(-16384.0, 16384.0)
    }

    fn cube() -> Brush {
        crate::shapes::create_cuboid(
            &world(),
            &Bbox3d::new(
                Vec3d::new(-32.0, -32.0, -32.0),
                Vec3d::new(32.0, 32.0, 32.0),
            ),
            "stone",
        )
        .expect("a cube")
    }

    #[test]
    fn nearby_facets_keep_distinct_source_faces_for_export() {
        let mut faces = cube().faces().to_vec();
        let top = faces
            .iter()
            .position(|face| face.plane.normal.z() > 0.99)
            .unwrap();
        let mut other = faces[top].clone();
        faces[top].plane.normal = vec::normalize(Vec3d::new(0.01, 0.0, 1.0));
        other.plane.normal = vec::normalize(Vec3d::new(-0.01, 0.0, 1.0));
        faces[top].material_name = "left".into();
        other.material_name = "right".into();
        faces.push(other);
        let brush = Brush::from_faces(&world(), faces).unwrap();
        let mut sources = std::collections::HashSet::new();
        brush.for_each_geometry_face(|index, face, polygon| {
            let source = brush.source_face_index(index).unwrap();
            assert!(sources.insert(source), "two facets map to one source face");
            assert_eq!(brush.face_polygon(source).unwrap(), polygon);
            assert!(polygon
                .iter()
                .all(|p| (vec::dot(face.plane.normal, *p) - face.plane.distance).abs() < 1e-6));
        });
        assert_eq!(sources.len(), 7);
    }

    /// The index of the face pointing the given way.
    fn face_facing(brush: &Brush, direction: Vec3d) -> usize {
        brush
            .faces()
            .iter()
            .position(|face| vec::dot(face.normal(), direction) > 0.99)
            .expect("a face that way")
    }

    #[test]
    fn an_inset_with_no_depth_tapers_the_solid() {
        let brush = cube();
        let top = face_facing(&brush, Vec3d::new(0.0, 0.0, 1.0));
        let tapered = brush.bevel_face(top, 8.0, 0.0).expect("it bevels");

        // The top has come in by eight all round; the bottom has not moved.
        assert_eq!(tapered.bounds(), brush.bounds());
        let corners = tapered.vertex_positions();
        let at_the_top: Vec<&Vec3d> = corners.iter().filter(|c| c.z() > 31.0).collect();
        assert_eq!(at_the_top.len(), 4, "the top is still a quad");
        assert!(at_the_top
            .iter()
            .all(|c| (c.x().abs() - 24.0).abs() < 0.001));
        let at_the_bottom: Vec<&Vec3d> = corners.iter().filter(|c| c.z() < -31.0).collect();
        assert!(at_the_bottom
            .iter()
            .all(|c| (c.x().abs() - 32.0).abs() < 0.001));
    }

    #[test]
    fn an_inset_with_depth_raises_a_panel_with_sloped_sides() {
        let brush = cube();
        let top = face_facing(&brush, Vec3d::new(0.0, 0.0, 1.0));
        let raised = brush.bevel_face(top, 8.0, 16.0).expect("it bevels");
        assert!(
            (raised.bounds().max.z() - 48.0).abs() < 0.001,
            "the top went up"
        );
        // Four sloped faces, the six the cube had, and no others.
        assert_eq!(raised.faces().len(), 6);
    }

    #[test]
    fn an_inset_that_eats_the_face_is_refused() {
        let brush = cube();
        let top = face_facing(&brush, Vec3d::new(0.0, 0.0, 1.0));
        assert!(brush.bevel_face(top, 64.0, 0.0).is_err());
        // And the brush is untouched, since nothing was returned to put back.
        assert_eq!(brush.vertex_positions().len(), 8);
    }

    #[test]
    fn a_panel_stands_on_the_face_it_was_made_from() {
        let brush = cube();
        let top = face_facing(&brush, Vec3d::new(0.0, 0.0, 1.0));
        let panel = brush.panel_on_face(top, 8.0, 16.0).expect("a panel");

        let bounds = panel.bounds();
        assert!(
            (bounds.min.z() - 32.0).abs() < 0.001,
            "it starts on the face"
        );
        assert!(
            (bounds.max.z() - 48.0).abs() < 0.001,
            "and is as deep as asked"
        );
        assert!(
            (bounds.size().x() - 48.0).abs() < 0.001,
            "drawn in by eight all round"
        );
        assert_eq!(
            panel.faces()[0].material_name,
            "stone",
            "it wears what the face wore"
        );
    }

    #[test]
    fn a_recess_is_the_same_shape_going_the_other_way() {
        let brush = cube();
        let top = face_facing(&brush, Vec3d::new(0.0, 0.0, 1.0));
        let tool = brush.panel_on_face(top, 8.0, -16.0).expect("a recess");
        assert!((tool.bounds().min.z() - 16.0).abs() < 0.001);
        assert!((tool.bounds().max.z() - 32.0).abs() < 0.001);

        // Taking it out of the cube leaves a tray: the walls and the floor under it.
        let left = crate::csg::subtract(&brush, &tool);
        assert!(!left.is_empty());
        assert!(!left
            .iter()
            .any(|piece| piece.contains_point(Vec3d::new(0.0, 0.0, 24.0))));
    }
}
