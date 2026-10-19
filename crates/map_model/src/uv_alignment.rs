//! Aligning, justifying and fitting a material on a face.
//!
//! UV alignment rotates a material along face edges, justification moves it against a face side,
//! and fitting scales it to a whole number of repeats.
//!
//! Every one of them can be asked more than once. That is [`UvPolicy`]: `Best` gives the
//! nearest sensible answer, while `Next` and `Prev` step through the others -- the next
//! edge round the face, the next repeat along the axis, the next whole number of copies --
//! so pressing the button again keeps offering choices instead of doing nothing.
//!
//! These functions return attribute values without mutating faces, leaving undo transaction
//! boundaries to the caller.
//!
//! A face's geometry lives on the brush rather than on the face, so the polygon is passed
//! in, as it is for [`crate::brush_face::BrushFace::fit_uv_to_face`].

use geometry_math::scalar;
use geometry_math::vec::{self, Vec2f, Vec3d};

use crate::brush_face::BrushFace;
use crate::uv::safe_scale;

/// Which of the answers a repeated press gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UvPolicy {
    /// The nearest sensible answer.
    Best,
    /// The one after whatever the face already has, so a press cycles forwards.
    Next,
    /// The one before it.
    Prev,
}

/// What "fit" means when the material is larger than the face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UvFitMode {
    /// Scale it down until the whole material shows on the face.
    FitToFace,
    /// Keep the 1/n subdivision, which is what a trim sheet wants: a strip of the sheet
    /// covers the face rather than the sheet being squashed onto it.
    TrimSheet,
}

/// Which of the material's two axes a command works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UvAxis {
    /// Across the material.
    U,
    /// Down it.
    V,
}

/// Which end of that axis: the far side or the near one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UvSign {
    /// The far end of the axis.
    Plus,
    /// The near end.
    Minus,
}

impl UvAxis {
    /// The axis as a unit vector in UV space.
    fn unit(self) -> Vec2f {
        match self {
            UvAxis::U => Vec2f::new(1.0, 0.0),
            UvAxis::V => Vec2f::new(0.0, 1.0),
        }
    }

    /// This axis's component of a UV pair.
    fn of(self, uv: Vec2f) -> f32 {
        vec::dot(uv, self.unit())
    }
}

impl UvSign {
    fn factor(self) -> f32 {
        match self {
            UvSign::Plus => 1.0,
            UvSign::Minus => -1.0,
        }
    }

    fn other(self) -> Self {
        match self {
            UvSign::Plus => UvSign::Minus,
            UvSign::Minus => UvSign::Plus,
        }
    }
}

/// How close two of these numbers have to be to count as the same, `Cf::almost_zero`.
const EPSILON: f32 = 0.001;

/// A point of the face in UV space, with the given offset and scale applied.
fn to_uv(face: &BrushFace, point: Vec3d, offset: Vec2f, scale: Vec2f) -> Vec2f {
    let (u_axis, v_axis) = face
        .uv_coord_system
        .axes(face.normal(), &face.uv_attributes);
    Vec2f::new(
        (vec::dot(point, u_axis) / f64::from(safe_scale(scale.x()))) as f32,
        (vec::dot(point, v_axis) / f64::from(safe_scale(scale.y()))) as f32,
    ) + offset
}

/// How far a vertex reaches along one UV axis, in the direction `sign` points,
/// `makeVertexToUvAxisTransform`. The face's own scale counts and its offset does not:
/// this is about where the geometry is, not where the material currently sits.
fn along(face: &BrushFace, vertex: Vec3d, axis: UvAxis, sign: UvSign) -> f32 {
    let uv = to_uv(face, vertex, Vec2f::zero(), face.uv_attributes.scale);
    axis.of(uv) * sign.factor()
}

/// Returns the smallest and largest values, or `None` when all values are equal.
fn extent(values: &[f32]) -> Option<(f32, f32)> {
    let min = values.iter().copied().fold(f32::INFINITY, f32::min);
    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    (min.is_finite() && max.is_finite() && max > min).then_some((min, max))
}

/// An angle brought into 0..360.
fn normalize_angle(degrees: f32) -> f32 {
    scalar::correct(degrees.rem_euclid(360.0), 0, EPSILON)
}

/// An offset brought into 0..length, so two offsets a whole material apart compare equal.
fn normalize_offset(offset: f32, length: f32) -> f32 {
    if length <= 0.0 {
        return offset;
    }
    let value = scalar::correct(offset % length, 0, EPSILON);
    if value < 0.0 {
        value + length
    } else {
        value
    }
}

/// The next scale factor to try, `findNextScaleFactor`: a whole number of copies when the
/// material is smaller than the face, a whole fraction when it is larger.
fn next_scale_factor(factor: f32, policy: UvPolicy) -> f32 {
    if factor.abs() < EPSILON {
        return 1.0;
    }
    let sign = if factor < 0.0 { -1.0 } else { 1.0 };
    let magnitude = factor.abs();
    let next = match policy {
        UvPolicy::Best => {
            if magnitude >= 1.0 {
                magnitude.round()
            } else {
                1.0 / (1.0 / magnitude).round()
            }
        }
        UvPolicy::Next => {
            if magnitude >= 1.0 {
                next_integer(magnitude)
            } else {
                1.0 / prev_integer(1.0 / magnitude)
            }
        }
        UvPolicy::Prev => {
            if (magnitude - 1.0).abs() < EPSILON {
                0.5
            } else if magnitude > 1.0 {
                prev_integer(magnitude)
            } else {
                1.0 / next_integer(1.0 / magnitude)
            }
        }
    };
    sign * next
}

/// The next whole number strictly above `value`, `geometry_math::next_integer`.
fn next_integer(value: f32) -> f32 {
    let up = value.ceil();
    if (up - value).abs() < EPSILON {
        up + 1.0
    } else {
        up
    }
}

/// The next whole number strictly below `value`, and never below one.
fn prev_integer(value: f32) -> f32 {
    let down = value.floor();
    let result = if (value - down).abs() < EPSILON {
        down - 1.0
    } else {
        down
    };
    result.max(1.0)
}

/// The face's edges as unit vectors in UV space, in the order the polygon gives them.
fn edge_directions(face: &BrushFace, polygon: &[Vec3d]) -> Vec<Vec2f> {
    let offset = face.uv_attributes.offset;
    let unit = Vec2f::new(1.0, 1.0);
    (0..polygon.len())
        .filter_map(|at| {
            let start = to_uv(face, polygon[at], offset, unit);
            let end = to_uv(face, polygon[(at + 1) % polygon.len()], offset, unit);
            let delta = end - start;
            (vec::length(delta) > EPSILON).then(|| vec::normalize(delta))
        })
        .collect()
}

/// The edge the material should run along, and whether it already does,
/// `findEdgeToAlignTo`: the edge closest to the U axis, or the one after or before it
/// when the material is already on that edge and the caller asked to cycle.
fn edge_to_align_to(
    face: &BrushFace,
    polygon: &[Vec3d],
    policy: UvPolicy,
) -> Option<(Vec2f, bool)> {
    let edges = edge_directions(face, polygon);
    if edges.is_empty() {
        return None;
    }
    let u = Vec2f::new(1.0, 0.0);
    let best = (0..edges.len())
        .max_by(|a, b| vec::dot(edges[*a], u).total_cmp(&vec::dot(edges[*b], u)))?;
    // `Cd::angle_epsilon` is coarser than the almost-zero used elsewhere: this is a
    // question about a direction, not about a length.
    let exact = (vec::dot(edges[best], u) - 1.0).abs() < 0.001;
    let at = match policy {
        UvPolicy::Best => best,
        UvPolicy::Next if exact => (best + 1) % edges.len(),
        UvPolicy::Prev if exact => (best + edges.len() - 1) % edges.len(),
        _ => best,
    };
    Some((edges[at], exact))
}

/// The rotation that would bring `point` -- a direction in UV space -- onto the U axis.
///
/// Measures rotation about the face normal for both UV systems. The resulting angle is subtracted
/// when updating material rotation.
fn measure_angle(face: &BrushFace, point: Vec2f) -> f32 {
    let rotation = face.uv_attributes.rotation;
    match face.uv_coord_system {
        crate::uv::UvCoordSystem::Paraxial => {
            let turned = turn(point, -rotation);
            let angle = vec::measure_angle_3d(
                vec::normalize(Vec3d::new(f64::from(turned.x()), f64::from(turned.y()), 0.0)),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 0.0, 1.0),
            );
            scalar::to_degrees(std::f64::consts::TAU - angle) as f32
        }
        crate::uv::UvCoordSystem::Parallel { .. } => {
            let angle = vec::measure_angle_3d(
                vec::normalize(Vec3d::new(f64::from(point.x()), f64::from(point.y()), 0.0)),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 0.0, 1.0),
            );
            rotation - scalar::to_degrees(angle) as f32
        }
    }
}

/// Rotates a UV vector about Z by an angle in degrees.
fn turn(point: Vec2f, degrees: f32) -> Vec2f {
    let radians = degrees.to_radians();
    let (sin, cos) = (radians.sin(), radians.cos());
    Vec2f::new(
        point.x() * cos - point.y() * sin,
        point.x() * sin + point.y() * cos,
    )
}

/// How far apart the face's ends are along one UV axis, and how long the material is
/// along it.
fn lengths(
    face: &BrushFace,
    polygon: &[Vec3d],
    material_size: Vec2f,
    axis: UvAxis,
    sign: UvSign,
) -> Option<(f32, f32, f32)> {
    let distances: Vec<f32> = polygon
        .iter()
        .map(|vertex| along(face, *vertex, axis, sign))
        .collect();
    let (min, max) = extent(&distances)?;
    Some((max - min, axis.of(material_size), max))
}

/// How many whole copies of the material fit the face along this axis, which is what
/// gives justify its several answers: a material four times the face's length can sit at
/// any of four offsets and still look justified.
fn subdivisions(material_length: f32, face_length: f32) -> usize {
    if face_length <= 0.0 || material_length <= 0.0 {
        return 1;
    }
    let remainder = material_length % face_length;
    if remainder.abs() < EPSILON || (remainder - face_length).abs() < EPSILON {
        (material_length / face_length).round().max(1.0) as usize
    } else {
        1
    }
}

/// The offsets that would leave the material justified against this side of the face.
fn justified_offsets(
    face: &BrushFace,
    polygon: &[Vec3d],
    material_size: Vec2f,
    axis: UvAxis,
    sign: UvSign,
) -> Option<Vec<f32>> {
    let (face_length, material_length, max) = lengths(face, polygon, material_size, axis, sign)?;
    let count = subdivisions(material_length, face_length);
    let step = material_length / count as f32;
    let base = -sign.factor() * max;
    Some(
        (0..count)
            .map(|division| normalize_offset(division as f32 * step + base, material_length))
            .collect(),
    )
}

/// Whether the material already runs along one of the face's edges, `isAligned`.
pub fn is_aligned(face: &BrushFace, polygon: &[Vec3d]) -> bool {
    edge_to_align_to(face, polygon, UvPolicy::Best).is_some_and(|(_, exact)| exact)
}

/// Whether the material already sits against this side of the face, `isJustified`.
pub fn is_justified(
    face: &BrushFace,
    polygon: &[Vec3d],
    material_size: Vec2f,
    axis: UvAxis,
    sign: UvSign,
) -> bool {
    let Some(offsets) = justified_offsets(face, polygon, material_size, axis, sign) else {
        return false;
    };
    let material_length = axis.of(material_size);
    let current = normalize_offset(axis.of(face.uv_attributes.offset), material_length);
    offsets
        .iter()
        .any(|offset| (offset - current).abs() < EPSILON)
}

/// Whether the material is already scaled to fit the face along this axis, `isFitted`.
pub fn is_fitted(
    face: &BrushFace,
    polygon: &[Vec3d],
    material_size: Vec2f,
    axis: UvAxis,
) -> bool {
    let Some(wanted) = fit(
        face,
        polygon,
        material_size,
        axis,
        UvPolicy::Best,
        UvFitMode::FitToFace,
    ) else {
        return false;
    };
    (axis.of(face.uv_attributes.scale) - wanted).abs() < EPSILON
}

/// The vertex a scale should hold still, `anchorVertex`: the far corner along the axis,
/// on whichever side the material is already justified against.
pub fn anchor_vertex(
    face: &BrushFace,
    polygon: &[Vec3d],
    material_size: Vec2f,
    axis: UvAxis,
    preferred: UvSign,
) -> Option<Vec3d> {
    let furthest = |sign: UvSign| {
        polygon
            .iter()
            .copied()
            .max_by(|a, b| along(face, *a, axis, sign).total_cmp(&along(face, *b, axis, sign)))
    };
    for sign in [preferred, preferred.other()] {
        if is_justified(face, polygon, material_size, axis, sign) {
            return furthest(sign);
        }
    }
    furthest(preferred)
}

/// The rotation that lays the material along one of the face's edges, `align`.
pub fn align(face: &BrushFace, polygon: &[Vec3d], policy: UvPolicy) -> Option<f32> {
    let (edge, _) = edge_to_align_to(face, polygon, policy)?;
    Some(normalize_angle(measure_angle(face, edge)))
}

/// The offset that pushes the material against one side of the face, `justify`.
pub fn justify(
    face: &BrushFace,
    polygon: &[Vec3d],
    material_size: Vec2f,
    axis: UvAxis,
    sign: UvSign,
    policy: UvPolicy,
) -> Option<f32> {
    let offsets = justified_offsets(face, polygon, material_size, axis, sign)?;
    if offsets.is_empty() {
        return None;
    }
    let material_length = axis.of(material_size);
    let current = normalize_offset(axis.of(face.uv_attributes.offset), material_length);
    let best = (0..offsets.len()).min_by(|a, b| {
        (offsets[*a] - current)
            .abs()
            .total_cmp(&(offsets[*b] - current).abs())
    })?;
    let exact = (offsets[best] - current).abs() < EPSILON;
    Some(match policy {
        UvPolicy::Best => offsets[0],
        UvPolicy::Next if exact => offsets[(best + 1) % offsets.len()],
        UvPolicy::Prev if exact => offsets[(best + offsets.len() - 1) % offsets.len()],
        _ => offsets[0],
    })
}

/// The scale that fits a whole number of copies of the material across the face, `fit`.
pub fn fit(
    face: &BrushFace,
    polygon: &[Vec3d],
    material_size: Vec2f,
    axis: UvAxis,
    policy: UvPolicy,
    mode: UvFitMode,
) -> Option<f32> {
    let unit = Vec2f::new(1.0, 1.0);
    let distances: Vec<f32> = polygon
        .iter()
        .map(|vertex| axis.of(to_uv(face, *vertex, Vec2f::zero(), unit)))
        .collect();
    let (min, max) = extent(&distances)?;
    let face_length = max - min;
    let material_length = axis.of(material_size);
    if material_length <= 0.0 {
        return None;
    }
    let current = axis.of(face.uv_attributes.scale);

    // A material larger than the face is scaled down until all of it shows, unless the
    // caller wants the 1/nth subdivision a trim sheet is drawn in.
    if mode == UvFitMode::FitToFace && material_length > face_length {
        let sign = if current < 0.0 { -1.0 } else { 1.0 };
        return Some(sign * face_length / material_length);
    }

    let factor = current * material_length / face_length;
    Some(next_scale_factor(factor, policy) * face_length / material_length)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uv::{UvAttributes, UvCoordSystem};

    /// The front cube face has normal -Y, with U along X and V down Z.
    fn front_face(system: UvCoordSystem) -> (BrushFace, Vec<Vec3d>) {
        let polygon = vec![
            Vec3d::new(-32.0, -32.0, 32.0),
            Vec3d::new(32.0, -32.0, 32.0),
            Vec3d::new(32.0, -32.0, -32.0),
            Vec3d::new(-32.0, -32.0, -32.0),
        ];
        let face = BrushFace::from_points(
            [polygon[0], polygon[1], polygon[2]],
            "material",
            system,
            UvAttributes::default(),
        )
        .expect("three points define the face");
        (face, polygon)
    }

    fn parallel() -> UvCoordSystem {
        UvCoordSystem::Parallel {
            u_axis: Vec3d::new(1.0, 0.0, 0.0),
            v_axis: Vec3d::new(0.0, 0.0, -1.0),
        }
    }

    const SIXTY_FOUR: Vec2f = Vec2f { v: [64.0, 64.0] };

    #[test]
    fn a_square_face_with_a_square_material_is_already_aligned() {
        for system in [UvCoordSystem::Paraxial, parallel()] {
            let (face, polygon) = front_face(system);
            assert!(is_aligned(&face, &polygon), "{system:?}");
        }
    }

    #[test]
    fn align_turns_the_material_onto_an_edge_whatever_it_started_at() {
        // The property that matters: whatever the material is turned to, setting the
        // rotation `align` gives leaves the face aligned.
        for system in [UvCoordSystem::Paraxial, parallel()] {
            for start in [7.0, 15.0, 33.0, 45.0, 100.0, 200.0, 350.0] {
                let (mut face, polygon) = front_face(system);
                face.rotate_uv(start);
                assert!(!is_aligned(&face, &polygon) || start % 90.0 == 0.0);

                let target = align(&face, &polygon, UvPolicy::Best).expect("an angle");
                face.rotate_uv(target - face.uv_attributes.rotation);
                assert!(
                    is_aligned(&face, &polygon),
                    "{system:?} from {start}: align said {target}"
                );
            }
        }
    }

    #[test]
    fn a_quarter_turn_is_still_aligned() {
        for turn in [90.0, 180.0, 270.0] {
            let (mut face, polygon) = front_face(UvCoordSystem::Paraxial);
            face.rotate_uv(turn);
            assert!(is_aligned(&face, &polygon), "{turn} degrees");
        }
    }

    #[test]
    fn justifying_puts_the_materials_edge_on_the_faces_edge() {
        let (mut face, polygon) = front_face(parallel());
        face.uv_attributes.offset = Vec2f::new(7.0, 11.0);
        assert!(!is_justified(&face, &polygon, SIXTY_FOUR, UvAxis::U, UvSign::Plus));

        let offset =
            justify(&face, &polygon, SIXTY_FOUR, UvAxis::U, UvSign::Plus, UvPolicy::Best)
                .expect("an offset");
        face.uv_attributes.offset = Vec2f::new(offset, face.uv_attributes.offset.y());
        assert!(is_justified(&face, &polygon, SIXTY_FOUR, UvAxis::U, UvSign::Plus));

        // And the far corner of the face now sits on a whole multiple of the material.
        let at = to_uv(&face, Vec3d::new(32.0, -32.0, 32.0), face.uv_attributes.offset, face.uv_attributes.scale);
        assert!(
            (normalize_offset(at.x(), 64.0)).abs() < 0.01,
            "the corner is at {at:?}"
        );
    }

    #[test]
    fn a_material_the_size_of_the_face_has_one_justified_offset() {
        // The face is 64 across and so is the material: one way to sit against its edge.
        let (face, polygon) = front_face(parallel());
        let offsets =
            justified_offsets(&face, &polygon, SIXTY_FOUR, UvAxis::U, UvSign::Plus).unwrap();
        assert_eq!(offsets.len(), 1);

        // A material four times as long can sit at any of four.
        let long = Vec2f::new(256.0, 64.0);
        let offsets = justified_offsets(&face, &polygon, long, UvAxis::U, UvSign::Plus).unwrap();
        assert_eq!(offsets.len(), 4);
        let mut sorted = offsets.clone();
        sorted.sort_by(f32::total_cmp);
        sorted.dedup();
        assert_eq!(sorted.len(), 4, "four different offsets: {offsets:?}");
    }

    #[test]
    fn fitting_scales_a_material_larger_than_the_face_down_to_it() {
        let (face, polygon) = front_face(parallel());
        // A 128-wide material on a 64-wide face fits at half scale.
        let size = Vec2f::new(128.0, 64.0);
        let scale = fit(
            &face,
            &polygon,
            size,
            UvAxis::U,
            UvPolicy::Best,
            UvFitMode::FitToFace,
        )
        .expect("a scale");
        assert!((scale - 0.5).abs() < 1e-5, "{scale}");
    }

    #[test]
    fn a_trim_sheet_keeps_its_subdivision() {
        let (face, polygon) = front_face(parallel());
        let size = Vec2f::new(128.0, 64.0);
        let scale = fit(
            &face,
            &polygon,
            size,
            UvAxis::U,
            UvPolicy::Best,
            UvFitMode::TrimSheet,
        )
        .expect("a scale");
        // The sheet is not squashed onto the face: a whole number of copies still covers
        // it, which at scale 1 it already does (two half-copies is not it -- 0.5 is).
        assert!(scale > 0.0);
        assert!((scale - 0.5).abs() < 1e-5 || (scale - 1.0).abs() < 1e-5, "{scale}");
    }

    #[test]
    fn fitting_a_small_material_lays_a_whole_number_of_copies_across_the_face() {
        let (mut face, polygon) = front_face(parallel());
        // A 16-wide material on a 64-wide face: four copies at scale 1, and the fit
        // rounds an awkward scale to the nearest whole count.
        let size = Vec2f::new(16.0, 64.0);
        face.uv_attributes.scale = Vec2f::new(0.9, 1.0);
        let scale = fit(
            &face,
            &polygon,
            size,
            UvAxis::U,
            UvPolicy::Best,
            UvFitMode::FitToFace,
        )
        .expect("a scale");
        let copies = 64.0 / (16.0 * scale);
        assert!((copies - copies.round()).abs() < 1e-4, "{copies} copies");
        assert!(is_fitted(
            &{
                let mut fitted = face.clone();
                fitted.uv_attributes.scale = Vec2f::new(scale, 1.0);
                fitted
            },
            &polygon,
            size,
            UvAxis::U
        ));
    }

    #[test]
    fn stepping_the_fit_offers_a_different_count_each_time() {
        let (mut face, polygon) = front_face(parallel());
        let size = Vec2f::new(16.0, 64.0);
        face.uv_attributes.scale = Vec2f::new(1.0, 1.0);

        let next = fit(&face, &polygon, size, UvAxis::U, UvPolicy::Next, UvFitMode::FitToFace)
            .expect("a scale");
        let prev = fit(&face, &polygon, size, UvAxis::U, UvPolicy::Prev, UvFitMode::FitToFace)
            .expect("a scale");
        assert!(next != prev, "next and previous give the same answer");
        // Four copies now. The factor a step moves is how much of the face one copy
        // covers, so "next" is a larger copy and therefore fewer of them: three, and
        // "previous" five.
        assert!((64.0 / (16.0 * next) - 3.0).abs() < 1e-4, "{next}");
        assert!((64.0 / (16.0 * prev) - 5.0).abs() < 1e-4, "{prev}");
    }

    #[test]
    fn the_anchor_is_the_corner_the_material_is_justified_against() {
        let (mut face, polygon) = front_face(parallel());
        let offset = justify(
            &face,
            &polygon,
            SIXTY_FOUR,
            UvAxis::U,
            UvSign::Plus,
            UvPolicy::Best,
        )
        .expect("an offset");
        face.uv_attributes.offset = Vec2f::new(offset, 0.0);

        let anchor = anchor_vertex(&face, &polygon, SIXTY_FOUR, UvAxis::U, UvSign::Plus)
            .expect("a vertex");
        assert_eq!(anchor.x(), 32.0, "the far corner along U");
    }

    #[test]
    fn a_degenerate_face_has_nothing_to_align_to() {
        let (face, _) = front_face(parallel());
        let point = vec![Vec3d::zero()];
        assert_eq!(align(&face, &point, UvPolicy::Best), None);
        assert_eq!(
            justify(&face, &point, SIXTY_FOUR, UvAxis::U, UvSign::Plus, UvPolicy::Best),
            None
        );
        assert_eq!(
            fit(&face, &point, SIXTY_FOUR, UvAxis::U, UvPolicy::Best, UvFitMode::FitToFace),
            None
        );
    }
}
