//! A convex polyhedron stored as a half-edge mesh.
//!
//! Every brush is a convex polyhedron, and editing operations construct updated polyhedra.
//!
//! ## Representation
//!
//! Vertices, half-edges, edges and faces use typed indices into owned arrays.
//!
//! Typed IDs prevent mixing vertex, edge and face indices. Indices belong to the
//! mesh that created them and must not be reused after rebuilding it.
//!
//! Edits rebuild the mesh from modified face polygons. Construction succeeds as a whole or leaves
//! the original mesh untouched.
//!
//! ## Precision
//!
//! Clipping interpolates along the edges it cuts, so a vertex built by clipping carries
//! the rounding error of that interpolation: clipping the world box down to a brush at
//! z = 32 can land at 31.999999999999996. That is a fraction of a millionth of a world
//! unit and harmless for rendering and collision, but it means vertex positions should be
//! compared with an epsilon rather than for exact equality. An editor that wants exact
//! coordinates snaps its brush vertices to the grid after an edit, which is a separate
//! step from the geometry here.
//!
//! ## Winding convention
//!
//! A face's vertices run **clockwise when the face is seen from outside** the polyhedron.
//! This is the convention the `.map` file format uses — the three points defining a face
//! are given clockwise as seen from the front — and it is what
//! [`geometry_math::plane::from_points`] assumes: it computes `cross(p3 - p1, p2 - p1)`, the reverse
//! of the usual cross product, so that clockwise vertices yield an outward normal.
//!
//! Getting this backwards is easy and the symptom is confusing, since a mesh wound the
//! wrong way still looks plausible: neighbouring faces end up traversing their shared
//! edge in the same direction instead of opposite ones, and
//! [`from_faces`](Polyhedron::from_faces) rejects it as
//! [`BuildError::DuplicateDirectedEdge`].

mod build;
mod clip;
mod hull;
mod query;

pub use build::BuildError;
pub use clip::{ClipError, ClipResult, FromPlanes};
pub use hull::HullError;

use geometry_math::bbox::Bbox;
use geometry_math::plane::Plane;
use geometry_math::scalar::Float;
use geometry_math::vec::Vector;

/// The index of a vertex in a [`Polyhedron`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexId(pub u32);

/// The index of a half-edge in a [`Polyhedron`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HalfEdgeId(pub u32);

/// The index of an edge in a [`Polyhedron`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(pub u32);

/// The index of a face in a [`Polyhedron`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaceId(pub u32);

macro_rules! impl_id {
    ($($t:ident),*) => {$(
        impl $t {
            /// Returns this id as an index into the corresponding arena.
            #[inline]
            pub fn index(self) -> usize {
                self.0 as usize
            }

            /// Creates an id from an arena index.
            #[inline]
            fn from_index(index: usize) -> Self {
                debug_assert!(index <= u32::MAX as usize, "arena index out of range");
                Self(index as u32)
            }
        }
    )*};
}

impl_id!(VertexId, HalfEdgeId, EdgeId, FaceId);

/// A vertex: a position, and one of the half-edges leaving it.
#[derive(Debug, Clone, Copy)]
pub struct Vertex<T> {
    /// The vertex position.
    pub position: Vector<T, 3>,
    /// One of the half-edges whose origin is this vertex.
    ///
    /// The others are reachable by walking `twin` then `next` around the vertex.
    pub leaving: HalfEdgeId,
}

/// A half-edge: one side of an edge, belonging to exactly one face.
///
/// The half-edges of a face form a cycle through `next`, wound clockwise when the face is
/// seen from outside the polyhedron. See the module documentation on winding.
#[derive(Debug, Clone, Copy)]
pub struct HalfEdge {
    /// The vertex this half-edge starts at.
    pub origin: VertexId,
    /// The edge this half-edge is one side of.
    pub edge: EdgeId,
    /// The face this half-edge belongs to.
    pub face: FaceId,
    /// The next half-edge around the face.
    pub next: HalfEdgeId,
    /// The previous half-edge around the face.
    pub previous: HalfEdgeId,
}

/// An edge: the pair of half-edges that face each other across it.
#[derive(Debug, Clone, Copy)]
pub struct Edge {
    /// One of the two half-edges.
    pub first: HalfEdgeId,
    /// The other half-edge, running the opposite way on the neighbouring face.
    pub second: HalfEdgeId,
}

/// A face: a plane, and one of the half-edges on its boundary.
#[derive(Debug, Clone, Copy)]
pub struct Face<T> {
    /// The face's plane, with its normal pointing out of the polyhedron.
    pub plane: Plane<T, 3>,
    /// One of the half-edges on this face's boundary.
    pub first: HalfEdgeId,
}

/// A convex polyhedron.
#[derive(Debug, Clone)]
pub struct Polyhedron<T> {
    vertices: Vec<Vertex<T>>,
    half_edges: Vec<HalfEdge>,
    edges: Vec<Edge>,
    faces: Vec<Face<T>>,
    bounds: Bbox<T, 3>,
}

impl<T: Float> Polyhedron<T> {
    /// Puts the solid through a transformation, without rebuilding it.
    ///
    /// An invertible transformation that keeps the orientation takes a convex solid to a
    /// convex solid with the same corners, edges and faces in the same arrangement --
    /// only their positions and planes change -- so there is nothing to work out. The
    /// planes come from the caller, which already has them from transforming the faces
    /// it keeps beside this; deriving them here would need the inverse transpose of the
    /// matrix and would be the same answer by a longer road.
    ///
    /// `plane_of_face` is asked for each face in turn. A transformation that mirrors or
    /// flattens the solid is not for this: the winding turns inside out or the volume
    /// goes, and either way the arrangement is not the one it had.
    pub fn transform(
        &mut self,
        matrix: &geometry_math::mat::Matrix<T, 4, 4>,
        plane_of_face: impl Fn(usize) -> Plane<T, 3>,
    ) {
        for vertex in &mut self.vertices {
            vertex.position = *matrix * vertex.position;
        }
        for (at, face) in self.faces.iter_mut().enumerate() {
            face.plane = plane_of_face(at);
        }
        // The box has to be measured again: a turn changes which corners are extreme.
        let mut bounds = match self.vertices.first() {
            Some(first) => Bbox::new(first.position, first.position),
            None => return,
        };
        for vertex in &self.vertices {
            bounds = bounds.merged_with_point(vertex.position);
        }
        self.bounds = bounds;
    }

    /// Moves the solid bodily, without rebuilding it.
    ///
    /// A translation takes a convex solid to a convex solid with the same corners,
    /// edges and faces in the same arrangement -- only their positions move -- so there
    /// is nothing to work out. Rebuilding it from its planes instead is what a drag of a
    /// large selection spends all its time on: 962 brushes take 116ms that way and under
    /// a millisecond this way.
    pub fn translate(&mut self, by: Vector<T, 3>) {
        for vertex in &mut self.vertices {
            vertex.position += by;
        }
        for face in &mut self.faces {
            // A plane's normal does not turn; its distance from the origin moves by how
            // far the solid moved along it.
            face.plane.distance += geometry_math::vec::dot(face.plane.normal, by);
        }
        self.bounds = Bbox::new(self.bounds.min + by, self.bounds.max + by);
    }

    /// Returns the polyhedron's vertices.
    #[inline]
    pub fn vertices(&self) -> &[Vertex<T>] {
        &self.vertices
    }

    /// Returns the polyhedron's half-edges.
    #[inline]
    pub fn half_edges(&self) -> &[HalfEdge] {
        &self.half_edges
    }

    /// Returns the polyhedron's edges.
    #[inline]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Returns the polyhedron's faces.
    #[inline]
    pub fn faces(&self) -> &[Face<T>] {
        &self.faces
    }

    /// Returns the vertex with the given id.
    #[inline]
    pub fn vertex(&self, id: VertexId) -> &Vertex<T> {
        &self.vertices[id.index()]
    }

    /// Returns the half-edge with the given id.
    #[inline]
    pub fn half_edge(&self, id: HalfEdgeId) -> &HalfEdge {
        &self.half_edges[id.index()]
    }

    /// Returns the edge with the given id.
    #[inline]
    pub fn edge(&self, id: EdgeId) -> &Edge {
        &self.edges[id.index()]
    }

    /// Returns the face with the given id.
    #[inline]
    pub fn face(&self, id: FaceId) -> &Face<T> {
        &self.faces[id.index()]
    }

    /// Returns the half-edge facing this one across its edge.
    #[inline]
    pub fn twin(&self, id: HalfEdgeId) -> HalfEdgeId {
        let edge = self.edge(self.half_edge(id).edge);
        if edge.first == id {
            edge.second
        } else {
            edge.first
        }
    }

    /// Returns the vertex this half-edge ends at.
    #[inline]
    pub fn destination(&self, id: HalfEdgeId) -> VertexId {
        self.half_edge(self.half_edge(id).next).origin
    }

    /// Returns the smallest axis aligned box containing this polyhedron.
    #[inline]
    pub fn bounds(&self) -> Bbox<T, 3> {
        self.bounds
    }

    /// Returns the number of vertices.
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the number of edges.
    #[inline]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns the number of faces.
    #[inline]
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Returns whether this polyhedron encloses a volume.
    ///
    /// A polyhedron needs at least four faces to do so; anything less is a point, an edge
    /// or a single polygon.
    #[inline]
    pub fn is_closed(&self) -> bool {
        self.faces.len() >= 4
    }
}
