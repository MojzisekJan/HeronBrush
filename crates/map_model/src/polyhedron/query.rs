//! Queries against a [`Polyhedron`].

use geometry_math::plane::Plane;
use geometry_math::scalar::Float;
use geometry_math::util::PlaneStatus;
use geometry_math::vec::{self, Vector};

use super::{FaceId, HalfEdgeId, Polyhedron, VertexId};

impl<T: Float> Polyhedron<T> {
    /// Returns the half-edges around the given face, in winding order.
    pub fn face_half_edges(&self, face: FaceId) -> Vec<HalfEdgeId> {
        let first = self.face(face).first;
        let mut result = vec![first];
        let mut current = self.half_edge(first).next;
        while current != first {
            result.push(current);
            current = self.half_edge(current).next;
        }
        result
    }

    /// Returns the vertices around the given face, in winding order.
    pub fn face_vertices(&self, face: FaceId) -> Vec<VertexId> {
        self.face_half_edges(face)
            .into_iter()
            .map(|id| self.half_edge(id).origin)
            .collect()
    }

    /// Returns the positions of the vertices around the given face, in winding order.
    pub fn face_positions(&self, face: FaceId) -> Vec<Vector<T, 3>> {
        let mut out = Vec::new();
        self.face_positions_into(face, &mut out);
        out
    }

    /// Fills `out` with the face's corner positions, clockwise as seen from outside,
    /// reusing its capacity. Walking every face of a mesh for rendering does this once
    /// per face; the allocating [`face_positions`](Self::face_positions),
    /// [`face_vertices`](Self::face_vertices) and
    /// [`face_half_edges`](Self::face_half_edges) built three `Vec`s to do the same.
    pub fn face_positions_into(&self, face: FaceId, out: &mut Vec<Vector<T, 3>>) {
        out.clear();
        let first = self.face(face).first;
        let mut current = first;
        loop {
            let edge = self.half_edge(current);
            out.push(self.vertex(edge.origin).position);
            current = edge.next;
            if current == first {
                break;
            }
        }
    }

    /// Returns the half-edges leaving the given vertex.
    ///
    /// Walking a half-edge's twin and then its next steps to the next half-edge around
    /// the vertex, so this circles the vertex once.
    pub fn vertex_half_edges(&self, vertex: VertexId) -> Vec<HalfEdgeId> {
        let first = self.vertex(vertex).leaving;
        let mut result = vec![first];
        let mut current = self.half_edge(self.twin(first)).next;
        while current != first {
            result.push(current);
            current = self.half_edge(self.twin(current)).next;
        }
        result
    }

    /// Returns the faces meeting at the given vertex.
    pub fn vertex_faces(&self, vertex: VertexId) -> Vec<FaceId> {
        self.vertex_half_edges(vertex)
            .into_iter()
            .map(|id| self.half_edge(id).face)
            .collect()
    }

    /// Returns whether the given point lies inside this polyhedron.
    ///
    /// A convex polyhedron is the intersection of its faces' half-spaces, so a point is
    /// inside exactly when it is behind or on every face.
    pub fn contains_point(&self, point: Vector<T, 3>, epsilon: T) -> bool {
        self.faces
            .iter()
            .all(|face| face.plane.point_status(point, epsilon) != PlaneStatus::Above)
    }

    /// Returns whether every vertex of `other` lies inside this polyhedron.
    pub fn contains(&self, other: &Self, epsilon: T) -> bool {
        other
            .vertices
            .iter()
            .all(|v| self.contains_point(v.position, epsilon))
    }

    /// Returns the total surface area.
    pub fn surface_area(&self) -> T {
        (0..self.faces.len())
            .map(|index| self.face_area(FaceId::from_index(index)))
            .fold(T::ZERO, |acc, area| acc + area)
    }

    /// Returns the area of the given face.
    ///
    /// The face is triangulated as a fan from its first vertex, which is valid because a
    /// convex polyhedron's faces are themselves convex.
    pub fn face_area(&self, face: FaceId) -> T {
        let positions = self.face_positions(face);
        if positions.len() < 3 {
            return T::ZERO;
        }

        let anchor = positions[0];
        let mut doubled = T::ZERO;
        for window in positions[1..].windows(2) {
            let triangle = vec::cross(window[0] - anchor, window[1] - anchor);
            doubled += vec::length(triangle);
        }
        doubled / T::TWO
    }

    /// Returns the plane of the given face.
    #[inline]
    pub fn face_plane(&self, face: FaceId) -> Plane<T, 3> {
        self.face(face).plane
    }

    /// Checks that the mesh's links are consistent, returning a description of the first
    /// problem found.
    ///
    /// Checks structural invariants for tests and debug assertions, outside performance-critical
    /// paths.
    pub fn check_invariant(&self) -> Result<(), String> {
        for (index, half_edge) in self.half_edges.iter().enumerate() {
            let id = HalfEdgeId::from_index(index);

            if self.half_edge(half_edge.next).previous != id {
                return Err(format!("half-edge {index}: next's previous is not itself"));
            }
            if self.half_edge(half_edge.previous).next != id {
                return Err(format!("half-edge {index}: previous's next is not itself"));
            }

            let twin = self.twin(id);
            if self.twin(twin) != id {
                return Err(format!("half-edge {index}: its twin's twin is not itself"));
            }
            if self.half_edge(twin).face == half_edge.face {
                return Err(format!(
                    "half-edge {index}: it and its twin are on the same face"
                ));
            }
            // The twin runs the other way, so it starts where this one ends.
            if self.half_edge(twin).origin != self.destination(id) {
                return Err(format!(
                    "half-edge {index}: its twin does not run back along it"
                ));
            }
        }

        for (index, face) in self.faces.iter().enumerate() {
            let id = FaceId::from_index(index);
            let half_edges = self.face_half_edges(id);
            if half_edges.len() < 3 {
                return Err(format!("face {index}: has fewer than 3 half-edges"));
            }
            for half_edge in &half_edges {
                if self.half_edge(*half_edge).face != id {
                    return Err(format!(
                        "face {index}: a boundary half-edge belongs elsewhere"
                    ));
                }
            }
            // Every vertex of a face must lie on that face's plane.
            for position in self.face_positions(id) {
                if face.plane.point_status(position, T::POINT_STATUS_EPSILON) != PlaneStatus::Inside
                {
                    return Err(format!("face {index}: a vertex is off its plane"));
                }
            }
        }

        for (index, vertex) in self.vertices.iter().enumerate() {
            let id = VertexId::from_index(index);
            if self.half_edge(vertex.leaving).origin != id {
                return Err(format!(
                    "vertex {index}: its leaving half-edge starts elsewhere"
                ));
            }
            if self.vertex_half_edges(id).len() < 3 {
                return Err(format!("vertex {index}: fewer than 3 edges meet here"));
            }
        }

        // Euler's formula holds for any polyhedron topologically equivalent to a sphere.
        let (v, e, f) = (self.vertices.len(), self.edges.len(), self.faces.len());
        if v + f != e + 2 {
            return Err(format!(
                "Euler's formula does not hold: {v} vertices + {f} faces != {e} edges + 2"
            ));
        }

        Ok(())
    }
}
