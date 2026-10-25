//! Finding what is under the pointer.
//!
//! Scene picking produces nearest-first ray hits for both orthographic and perspective views.

use geometry_math::abstract_line::AbstractLine;
use geometry_math::intersection;
use geometry_math::ray::Ray3d;
use geometry_math::vec::Vec3d;

use crate::node::{brushes_with_ids, NodeId, World};

/// Something a ray hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// The node that was hit.
    pub node: NodeId,
    /// Which face of that node's brush the ray entered through.
    ///
    /// This is an index into the brush's geometry faces, which is what
    /// [`Brush::geometry_faces`](crate::brush::Brush::geometry_faces) walks.
    pub face: usize,
    /// How far along the ray the hit is.
    pub distance: f64,
    /// Where the hit is, in world space.
    pub point: Vec3d,
}

/// The area of the convex `polygon` seen along `axis`: its shadow on the plane square to
/// that axis.
pub fn projected_area(polygon: &[Vec3d], axis: usize) -> f64 {
    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
    let n = polygon.len();
    (0..n)
        .map(|i| {
            let (a, b) = (polygon[i], polygon[(i + 1) % n]);
            a[u] * b[v] - b[u] * a[v]
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

/// The area of `bounds` seen along `axis`.
pub fn projected_box_area(bounds: &geometry_math::bbox::Bbox3d, axis: usize) -> f64 {
    let size = bounds.size();
    size[(axis + 1) % 3] * size[(axis + 2) % 3]
}

/// Returns everything the given ray hits, nearest first.
///
/// The ray is tested against each brush's bounding box before its faces, since most
/// brushes in a map are nowhere near any given ray.
pub fn pick(world: &World, ray: &Ray3d) -> Vec<Hit> {
    let mut hits: Vec<Hit> = brushes_with_ids(world)
        .into_iter()
        .filter(|(_, brush)| intersection::intersect_ray_bbox(ray, &brush.bounds()).is_some())
        .filter_map(|(node, brush)| {
            // The nearest face the ray enters through.
            let mut nearest: Option<(usize, f64)> = None;
            brush.for_each_geometry_face(|index, _, polygon| {
                if let Some(distance) = intersection::intersect_ray_polygon(ray, polygon) {
                    if nearest.is_none_or(|(_, best)| distance < best) {
                        nearest = Some((index, distance));
                    }
                }
            });
            nearest.map(|(face, distance)| Hit {
                node,
                face,
                distance,
                point: ray.point_at_distance(distance),
            })
        })
        .collect();

    hits.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits
}

/// Returns the nearest thing the given ray hits.
#[inline]
pub fn pick_nearest(world: &World, ray: &Ray3d) -> Option<Hit> {
    pick(world, ray).into_iter().next()
}
