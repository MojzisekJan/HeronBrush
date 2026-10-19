//! Geometry primitives. Everything here works in map units (the coordinates
//! written in the `.map`) and in f64. Scaling and the swizzle into Godot's
//! coordinate system happen once, at write time.

pub use glam::{DVec2 as Vec2, DVec3 as Vec3};

/// Distance below which two vertices are considered the same point.
pub const VERTEX_EPSILON: f64 = 0.008;

/// Half-width of the oversized quad each face starts as before being clipped.
pub const DEFAULT_HYPERPLANE_SIZE: f64 = 16384.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f64,
}

impl Plane {
    /// Builds a plane from the three points of a `.map` face, wound Quake's way,
    /// so the normal points out of the brush.
    pub fn from_points(p0: Vec3, p1: Vec3, p2: Vec3) -> Option<Plane> {
        let n = (p0 - p2).cross(p0 - p1);
        let len = n.length();
        if len < 1e-9 {
            return None;
        }
        let normal = n / len;
        Some(Plane {
            normal,
            d: normal.dot(p0),
        })
    }

    pub fn distance_to(&self, p: Vec3) -> f64 {
        self.normal.dot(p) - self.d
    }

    pub fn center(&self) -> Vec3 {
        self.normal * self.d
    }

    /// True when both planes describe the same surface facing the same way.
    pub fn is_equivalent(&self, other: &Plane, eps: f64) -> bool {
        self.normal.dot(other.normal) > 1.0 - eps && (self.d - other.d).abs() < eps
    }
}

/// Clips a convex polygon against a plane and keeps the half behind it. Points
/// within `eps` count as on it, which keeps near-coplanar faces from slivering.
pub fn clip_polygon(poly: &[Vec3], plane: &Plane, eps: f64) -> Vec<Vec3> {
    let mut winding = poly.to_vec();
    let mut scratch = Vec::new();
    clip_polygon_in_place(&mut winding, &mut scratch, plane, eps);
    winding
}

/// [`clip_polygon`] without the allocations: the result lands back in `poly`, with
/// `scratch` as the pass's working buffer. A run of clips hands the same two vectors
/// through every pass -- they trade places -- so the whole run allocates only as the
/// polygon outgrows what came before. This is the exporter's hottest loop: every face
/// of every brush is cut this way, once per sibling plane.
pub fn clip_polygon_in_place(
    poly: &mut Vec<Vec3>,
    scratch: &mut Vec<Vec3>,
    plane: &Plane,
    eps: f64,
) {
    if poly.is_empty() {
        return;
    }
    // Settle the two whole-sided cases in one pass; recomputing a distance later is
    // five flops, cheaper than keeping them in a heap-allocated list.
    let mut all_behind = true;
    let mut all_front = true;
    for p in poly.iter() {
        let d = plane.distance_to(*p);
        all_behind &= d <= eps;
        all_front &= d >= -eps;
        if !all_behind && !all_front {
            break;
        }
    }
    if all_behind {
        return;
    }
    if all_front {
        poly.clear();
        return;
    }

    scratch.clear();
    let mut di = plane.distance_to(poly[0]);
    for i in 0..poly.len() {
        let j = if i + 1 == poly.len() { 0 } else { i + 1 };
        let dj = plane.distance_to(poly[j]);

        if di <= eps {
            scratch.push(poly[i]);
        }
        // Only split the edge when the two ends sit on opposite sides for real.
        if (di > eps && dj < -eps) || (di < -eps && dj > eps) {
            let t = di / (di - dj);
            scratch.push(poly[i] + (poly[j] - poly[i]) * t);
        }
        di = dj;
    }
    std::mem::swap(poly, scratch);
}

/// The oversized square a face starts as, centred on the plane.
pub fn base_winding(plane: &Plane, size: f64) -> Vec<Vec3> {
    base_winding_around(plane, plane.center(), size)
}

/// A square on `plane`, `2 * size` on a side, centred on `around` pulled onto the plane.
///
/// Starting a clip from a square that already hugs the brush -- rather than one 100000
/// units across centred on the plane's foot -- keeps every intermediate polygon small,
/// which is what makes `corners_of` cheap on a large merged plane set.
pub fn base_winding_around(plane: &Plane, around: Vec3, size: f64) -> Vec<Vec3> {
    let mut out = Vec::with_capacity(4);
    base_winding_around_into(&mut out, plane, around, size);
    out
}

/// [`base_winding_around`] into a buffer the caller keeps, so a loop over many faces
/// starts each square in the same allocation.
pub fn base_winding_around_into(out: &mut Vec<Vec3>, plane: &Plane, around: Vec3, size: f64) {
    let up = if plane.normal.dot(Vec3::Z).abs() > 0.9 {
        Vec3::X
    } else {
        Vec3::Z
    };
    let right = plane.normal.cross(up).normalize();
    let forward = right.cross(plane.normal).normalize();
    // `around` projected onto the plane.
    let c = around - plane.normal * plane.distance_to(around);
    out.clear();
    out.extend_from_slice(&[
        c + right * size + forward * size,
        c - right * size + forward * size,
        c - right * size - forward * size,
        c + right * size - forward * size,
    ]);
}

/// Drops consecutive duplicates from a winding, wrapping around the end.
pub fn dedup_winding(winding: &[Vec3], eps: f64) -> Vec<Vec3> {
    let mut out = winding.to_vec();
    dedup_winding_in_place(&mut out, eps);
    out
}

/// [`dedup_winding`] on the vector itself, allocation-free.
pub fn dedup_winding_in_place(winding: &mut Vec<Vec3>, eps: f64) {
    winding.dedup_by(|current, kept| kept.distance_squared(*current) <= eps * eps);
    while winding.len() > 1 {
        let first = winding[0];
        let last = *winding.last().unwrap();
        if first.distance_squared(last) <= eps * eps {
            winding.pop();
        } else {
            break;
        }
    }
}

/// Sorts a coplanar point set counter-clockwise around its own centroid.
pub fn sort_winding(winding: &mut [Vec3], normal: Vec3) {
    if winding.len() < 3 {
        return;
    }
    let centroid = winding.iter().copied().fold(Vec3::ZERO, |a, b| a + b) / winding.len() as f64;
    let u_axis = (winding[1] - winding[0]).normalize();
    let v_axis = u_axis.cross(normal).normalize();
    winding.sort_by(|a, b| {
        let da = *a - centroid;
        let db = *b - centroid;
        let key_a = angle_key(da.dot(u_axis), da.dot(v_axis));
        let key_b = angle_key(db.dot(u_axis), db.dot(v_axis));
        key_a
            .partial_cmp(&key_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// A stand-in for `atan2(y, x)` that sorts the same but skips the trigonometry: the
/// "diamond angle", the position of the direction along the edge of the unit diamond,
/// shifted so it grows over (-pi, pi] exactly as `atan2` does. This comparator ran in
/// the profile's top handful before; the sort itself was not the cost, the `atan2` was.
fn angle_key(x: f64, y: f64) -> f64 {
    let d = x.abs() + y.abs();
    if d <= 0.0 {
        return 0.0;
    }
    let p = x / d;
    if y > 0.0 {
        1.0 - p // the upper half, 0 at +x rising to 2 at -x, as atan2 rises 0 to pi
    } else if y < 0.0 {
        p - 1.0 // the lower half, -2 just past -x rising to 0 at +x
    } else if x >= 0.0 {
        0.0 // exactly +x: atan2 says 0
    } else {
        2.0 // exactly -x: atan2 says +pi, the top of its range
    }
}

/// Snaps a coordinate to a grid, used to make vertices from neighbouring
/// brushes land on exactly the same value.
pub fn snap(v: Vec3, step: f64) -> Vec3 {
    if step <= 0.0 {
        return v;
    }
    Vec3::new(
        (v.x / step).round() * step,
        (v.y / step).round() * step,
        (v.z / step).round() * step,
    )
}

/// id Tech axes to Godot axes. Forward becomes -Z, up becomes +Y.
pub fn to_godot(v: Vec3) -> Vec3 {
    Vec3::new(v.y, v.z, v.x)
}

#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn from_points(points: impl IntoIterator<Item = Vec3>) -> Option<Aabb> {
        let mut it = points.into_iter();
        let first = it.next()?;
        let mut aabb = Aabb {
            min: first,
            max: first,
        };
        for p in it {
            aabb.min = aabb.min.min(p);
            aabb.max = aabb.max.max(p);
        }
        Some(aabb)
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_cube_planes() -> Vec<Plane> {
        vec![
            Plane {
                normal: Vec3::X,
                d: 1.0,
            },
            Plane {
                normal: -Vec3::X,
                d: 1.0,
            },
            Plane {
                normal: Vec3::Y,
                d: 1.0,
            },
            Plane {
                normal: -Vec3::Y,
                d: 1.0,
            },
            Plane {
                normal: Vec3::Z,
                d: 1.0,
            },
            Plane {
                normal: -Vec3::Z,
                d: 1.0,
            },
        ]
    }

    #[test]
    fn face_of_a_cube_clips_down_to_a_square() {
        let planes = unit_cube_planes();
        let mut winding = base_winding(&planes[0], 1024.0);
        for (i, p) in planes.iter().enumerate() {
            if i == 0 {
                continue;
            }
            winding = clip_polygon(&winding, p, 1e-6);
        }
        let winding = dedup_winding(&winding, VERTEX_EPSILON);
        assert_eq!(winding.len(), 4);
        for v in &winding {
            assert!((v.x - 1.0).abs() < 1e-6);
            assert!((v.y.abs() - 1.0).abs() < 1e-6);
            assert!((v.z.abs() - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn plane_from_map_points_faces_outward() {
        // A floor at z = 0 written the way a `.map` editor writes it.
        let p = Plane::from_points(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        )
        .unwrap();
        assert!((p.normal - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-9);
    }
}
