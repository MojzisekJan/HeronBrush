//! Building a [`Polyhedron`] from face polygons.
//!
//! The half-edge topology is derived rather than given: each face contributes one
//! half-edge per boundary edge, and half-edges running between the same two vertices in
//! opposite directions are paired into an edge. A closed convex polyhedron pairs up
//! exactly, which is what [`Polyhedron::from_faces`] checks.

use rustc_hash::FxHashMap;

use geometry_math::bbox::Bbox;
use geometry_math::plane;
use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vector};

use super::{Edge, EdgeId, Face, FaceId, HalfEdge, HalfEdgeId, Polyhedron, Vertex, VertexId};

/// Why a polyhedron could not be built from a set of face polygons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// Fewer than four faces were given, so they cannot enclose a volume.
    NotEnoughFaces {
        /// How many faces were given.
        given: usize,
    },
    /// A face had fewer than three distinct vertices.
    DegenerateFace {
        /// Which face, by its position in the input.
        face: usize,
        /// How many distinct vertices it had.
        vertices: usize,
    },
    /// A face's vertices are colinear or coincident, so they define no plane.
    FaceHasNoPlane {
        /// Which face, by its position in the input.
        face: usize,
    },
    /// The same directed edge appeared on two different faces.
    ///
    /// In a consistently wound mesh each directed edge belongs to exactly one face; a
    /// repeat means two faces are wound the same way where they meet.
    DuplicateDirectedEdge {
        /// The edge's endpoints.
        from: usize,
        /// The edge's endpoints.
        to: usize,
    },
    /// A half-edge had no counterpart running the other way, so the mesh has a hole.
    OpenEdge {
        /// The edge's endpoints.
        from: usize,
        /// The edge's endpoints.
        to: usize,
    },
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::NotEnoughFaces { given } => write!(
                f,
                "a polyhedron needs at least 4 faces to enclose a volume, got {given}"
            ),
            BuildError::DegenerateFace { face, vertices } => write!(
                f,
                "face {face} has {vertices} distinct vertices, which is fewer than 3"
            ),
            BuildError::FaceHasNoPlane { face } => {
                write!(f, "the vertices of face {face} do not define a plane")
            }
            BuildError::DuplicateDirectedEdge { from, to } => write!(
                f,
                "the edge from vertex {from} to {to} appears on more than one face, so the faces are not wound consistently"
            ),
            BuildError::OpenEdge { from, to } => write!(
                f,
                "the edge from vertex {from} to {to} has no neighbouring face, so the mesh is not closed"
            ),
        }
    }
}

impl std::error::Error for BuildError {}

impl<T: Float> Polyhedron<T> {
    /// Builds a polyhedron from its face polygons.
    ///
    /// Each face is a list of vertices wound clockwise when seen from outside the
    /// polyhedron, as described in the module documentation. Vertices closer together
    /// than `epsilon` are treated as the same vertex, which is how faces get stitched to
    /// their neighbours.
    pub fn from_faces(faces: &[Vec<Vector<T, 3>>], epsilon: T) -> Result<Self, BuildError> {
        if faces.len() < 4 {
            return Err(BuildError::NotEnoughFaces { given: faces.len() });
        }

        let mut builder = Builder::new(epsilon);
        for (index, face) in faces.iter().enumerate() {
            builder.add_face(index, face)?;
        }
        builder.finish()
    }

    /// Builds the polyhedron of an axis aligned box.
    ///
    /// This is the starting point for building a brush from planes: the box is the
    /// editable world, and each plane clips a piece off it.
    pub fn from_bbox(bounds: &Bbox<T, 3>) -> Result<Self, BuildError> {
        let faces: Vec<Vec<Vector<T, 3>>> = bounds
            .faces()
            .iter()
            .map(|face| face.vertices.to_vec())
            .collect();
        Self::from_faces(&faces, T::ALMOST_ZERO)
    }
}

/// Accumulates vertices and half-edges while faces are added, then ties them together.
struct Builder<T> {
    epsilon: T,
    positions: Vec<Vector<T, 3>>,
    vertex_leaving: Vec<Option<HalfEdgeId>>,
    half_edges: Vec<PartialHalfEdge>,
    faces: Vec<Face<T>>,
    /// Maps a directed vertex pair to the half-edge running along it, so a half-edge can
    /// find the one facing it.
    directed: FxHashMap<(u32, u32), HalfEdgeId>,
}

/// A half-edge before its edge has been identified.
struct PartialHalfEdge {
    origin: VertexId,
    face: FaceId,
    next: HalfEdgeId,
    previous: HalfEdgeId,
}

impl<T: Float> Builder<T> {
    fn new(epsilon: T) -> Self {
        Self {
            epsilon,
            positions: Vec::new(),
            vertex_leaving: Vec::new(),
            half_edges: Vec::new(),
            faces: Vec::new(),
            directed: FxHashMap::default(),
        }
    }

    /// Returns the id of the vertex at the given position, creating it if this is the
    /// first face to mention it.
    fn vertex_at(&mut self, position: Vector<T, 3>) -> VertexId {
        // A brush has a handful of vertices, so a linear scan beats any index we could
        // build, and it lets us match within an epsilon rather than exactly.
        for (index, existing) in self.positions.iter().enumerate() {
            if vec::squared_distance(*existing, position) <= self.epsilon * self.epsilon {
                return VertexId::from_index(index);
            }
        }
        self.positions.push(position);
        self.vertex_leaving.push(None);
        VertexId::from_index(self.positions.len() - 1)
    }

    fn add_face(&mut self, index: usize, polygon: &[Vector<T, 3>]) -> Result<(), BuildError> {
        // Resolve the polygon's points to vertex ids, dropping any that repeat the
        // previous one after snapping.
        let mut ring: Vec<VertexId> = Vec::with_capacity(polygon.len());
        for point in polygon {
            let id = self.vertex_at(*point);
            if ring.last() != Some(&id) {
                ring.push(id);
            }
        }
        if ring.len() > 1 && ring.first() == ring.last() {
            ring.pop();
        }
        if ring.len() < 3 {
            return Err(BuildError::DegenerateFace {
                face: index,
                vertices: ring.len(),
            });
        }

        let face_plane = plane::from_points(
            self.positions[ring[0].index()],
            self.positions[ring[1].index()],
            self.positions[ring[2].index()],
        )
        .ok_or(BuildError::FaceHasNoPlane { face: index })?;

        let face_id = FaceId::from_index(self.faces.len());
        let base = self.half_edges.len();
        let count = ring.len();

        for (offset, origin) in ring.iter().enumerate() {
            let id = HalfEdgeId::from_index(base + offset);
            let next = HalfEdgeId::from_index(base + (offset + 1) % count);
            let previous = HalfEdgeId::from_index(base + (offset + count - 1) % count);

            let destination = ring[(offset + 1) % count];
            let key = (origin.0, destination.0);
            if self.directed.insert(key, id).is_some() {
                return Err(BuildError::DuplicateDirectedEdge {
                    from: origin.index(),
                    to: destination.index(),
                });
            }

            self.half_edges.push(PartialHalfEdge {
                origin: *origin,
                face: face_id,
                next,
                previous,
            });
            if self.vertex_leaving[origin.index()].is_none() {
                self.vertex_leaving[origin.index()] = Some(id);
            }
        }

        self.faces.push(Face {
            plane: face_plane,
            first: HalfEdgeId::from_index(base),
        });
        Ok(())
    }

    /// Pairs up opposing half-edges into edges and assembles the finished polyhedron.
    fn finish(self) -> Result<Polyhedron<T>, BuildError> {
        let Self {
            positions,
            vertex_leaving,
            half_edges,
            faces,
            directed,
            ..
        } = self;

        let mut edges: Vec<Edge> = Vec::with_capacity(half_edges.len() / 2);
        let mut edge_of: Vec<Option<EdgeId>> = vec![None; half_edges.len()];

        for (index, half_edge) in half_edges.iter().enumerate() {
            let id = HalfEdgeId::from_index(index);
            if edge_of[index].is_some() {
                continue;
            }

            let from = half_edge.origin;
            let to = half_edges[half_edge.next.index()].origin;
            let twin = *directed.get(&(to.0, from.0)).ok_or(BuildError::OpenEdge {
                from: from.index(),
                to: to.index(),
            })?;

            let edge_id = EdgeId::from_index(edges.len());
            edges.push(Edge {
                first: id,
                second: twin,
            });
            edge_of[index] = Some(edge_id);
            edge_of[twin.index()] = Some(edge_id);
        }

        let half_edges: Vec<HalfEdge> = half_edges
            .into_iter()
            .zip(edge_of)
            .map(|(partial, edge)| HalfEdge {
                origin: partial.origin,
                edge: edge.expect("every half-edge was paired above"),
                face: partial.face,
                next: partial.next,
                previous: partial.previous,
            })
            .collect();

        let vertices: Vec<Vertex<T>> = positions
            .iter()
            .zip(&vertex_leaving)
            .map(|(position, leaving)| Vertex {
                position: *position,
                leaving: leaving.expect("every vertex is on at least one face"),
            })
            .collect();

        let bounds = Bbox::merge_all(positions.iter().copied())
            .expect("a polyhedron with four faces has vertices");

        Ok(Polyhedron {
            vertices,
            half_edges,
            edges,
            faces,
            bounds,
        })
    }
}
