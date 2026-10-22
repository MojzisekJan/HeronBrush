//! Sweeping: dragging a face along a path and filling in what it passes through.
//!
//! Every other tool moves geometry that is already there. This one makes geometry out of
//! a movement: take a face, say where it should end up, and the tool fills the space it
//! travelled through with brushes. A corridor, a staircase, a curved pipe -- all things
//! that are a face and a path rather than a shape.
//!
//! The path is a straight line, an arc, or an S-bend, and the sweep can repeat, each
//! repetition continuing from where the last one ended. What makes it hold together is
//! that a station's vertices depend only on which station it is: two neighbouring segments
//! compute the vertices they share identically, so the result stays watertight even when
//! the coordinates are being rounded.

use geometry_math::bbox::Bbox3d;
use geometry_math::mat::Mat4x4d;
use geometry_math::mat_ext;
use geometry_math::quat::Quatd;
use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vec3d};
use map_model::brush::Brush;
use map_model::polyhedron::Polyhedron;

/// Whether the stations land on whole coordinates.
///
/// `SweepAlignment`. A level is built on a grid, and a sweep that lands between grid lines
/// is a sweep whose result cannot be lined up with anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SweepAlignment {
    /// Round every station but the first onto whole coordinates.
    #[default]
    Integer,
    /// Leave them wherever the path puts them.
    Free,
}

/// The shape of the path the face travels along.
///
/// `SweepPathMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SweepPathMode {
    /// A circular arc, fitted so the face turns as it goes: a curved pipe.
    #[default]
    Arc,
    /// A straight line.
    Straight,
    /// A curve that leaves along the face's normal and arrives along the turned one.
    SBend,
}

/// What is being swept.
///
/// `SweepSource`.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepSource {
    /// The faces to sweep, as the rings of points they cover.
    pub faces: Vec<Vec<Vec3d>>,
    /// Optional source surface attributes, in face order.
    pub attributes: Vec<map_model::brush_face::BrushFace>,
    /// The middle of them all, which the path starts at.
    pub center: Vec3d,
    /// Which way they face, which the S-bend leaves along.
    pub normal: Vec3d,
}

/// Where the face ends up.
///
/// `SweepTransform`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepTransform {
    /// How far the face travels.
    pub translation: Vec3d,
    /// How far it turns on the way.
    pub rotation: Quatd,
    /// How much larger or smaller it becomes.
    pub scale: Vec3d,
}

impl Default for SweepTransform {
    fn default() -> Self {
        Self {
            translation: Vec3d::zero(),
            rotation: Quatd::from_axis_angle(Vec3d::new(0.0, 0.0, 1.0), 0.0),
            scale: Vec3d::new(1.0, 1.0, 1.0),
        }
    }
}

impl SweepTransform {
    /// Where the face ends up.
    pub fn destination_center(&self, source: &SweepSource) -> Vec3d {
        source.center + self.translation
    }

    /// The rotation, taken the short way round.
    ///
    /// `SweepTransform::effectiveRotation`. A quaternion's angle runs from 0 to a full
    /// turn, so a ring dragged 330 degrees would sweep all 330; past a half turn the
    /// smaller turn about the opposite axis is what was meant.
    pub fn effective_rotation(&self) -> Quatd {
        let theta = self.rotation.angle();
        if theta <= std::f64::consts::PI + f64::ALMOST_ZERO {
            return self.rotation;
        }

        // A full turn is the identity, and the axis is degenerate there.
        let short_angle = std::f64::consts::TAU - theta;
        if short_angle < f64::ALMOST_ZERO {
            Quatd::from_axis_angle(Vec3d::new(0.0, 0.0, 1.0), 0.0)
        } else {
            Quatd::from_axis_angle(-vec::normalize(self.rotation.axis()), short_angle)
        }
    }

    /// Returns whether this transform would sweep nothing.
    pub fn is_no_op(&self) -> bool {
        vec::is_zero(self.translation, f64::ALMOST_ZERO)
            && self.effective_rotation().angle() < f64::ALMOST_ZERO
            && vec::is_equal(&self.scale, &Vec3d::new(1.0, 1.0, 1.0), f64::ALMOST_ZERO)
    }
}

/// The transform and the parameters together: what the settings panel holds and what a
/// dragged destination cap changes, shared so the two stay in step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SweepInputs {
    /// Where the face goes.
    pub transform: SweepTransform,
    /// How the path between is shaped.
    pub parameters: SweepParameters,
}

/// How finely the sweep is cut up, and how many times it repeats.
///
/// `SweepParameters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepParameters {
    /// How many brushes each repetition is made of.
    pub segments: usize,
    /// How many times the whole sweep repeats, each continuing from the last.
    pub iterations: usize,
    /// The shape of the path.
    pub path_mode: SweepPathMode,
    /// Whether the stations land on whole coordinates.
    pub alignment: SweepAlignment,
}

impl Default for SweepParameters {
    fn default() -> Self {
        Self {
            segments: 8,
            iterations: 1,
            path_mode: SweepPathMode::default(),
            alignment: SweepAlignment::default(),
        }
    }
}

/// The point an arc revolves about, if there is one.
///
/// `arcPivot`. The face travels from one centre to another while turning by a known
/// angle; there is exactly one circle in the plane across the axis that does both, and
/// this finds its centre. Any travel along the axis itself becomes a helical lift rather
/// than part of the circle.
fn arc_pivot(source: &SweepSource, transform: &SweepTransform, rotation: &Quatd) -> Option<Vec3d> {
    let theta = rotation.angle();
    if theta.abs() < f64::ALMOST_ZERO {
        return None;
    }

    let axis = vec::normalize(rotation.axis());
    let c0 = source.center;
    let c1 = transform.destination_center(source);

    // Work in the plane perpendicular to the axis; the rise becomes a helix lift.
    let rise = vec::dot(c1 - c0, axis);
    let u1 = c1 - axis * rise;
    let chord = u1 - c0;
    let chord_length = vec::length(chord);
    if chord_length < f64::ALMOST_ZERO {
        // No lateral travel, so there is no circle to fit.
        return None;
    }

    let in_plane_perp = vec::cross(axis, chord);
    let perp_length = vec::length(in_plane_perp);
    if perp_length < f64::ALMOST_ZERO {
        return None;
    }
    let in_plane_perp = in_plane_perp / perp_length;

    let mid = (c0 + u1) * 0.5;
    let distance_mid_to_center = (chord_length * 0.5) / (theta * 0.5).tan();

    // Pick the candidate centre whose rotation by theta maps c0 onto u1.
    let turn = Quatd::from_axis_angle(axis, theta);
    let mut best_pivot = mid;
    let mut best_error = f64::MAX;
    for sign in [1.0, -1.0] {
        let pivot = mid + in_plane_perp * (distance_mid_to_center * sign);
        let mapped = pivot + turn * (c0 - pivot);
        let error = vec::squared_length(mapped - u1);
        if error < best_error {
            best_error = error;
            best_pivot = pivot;
        }
    }
    Some(best_pivot)
}

/// Where along an S-bend the face is at `t`.
///
/// `sBendOffset`: a cubic Hermite curve whose end tangents are the source's normal and the
/// turned one, which is what makes the face leave and arrive facing the way it points
/// rather than cutting the corner.
fn s_bend_offset(
    source: &SweepSource,
    transform: &SweepTransform,
    t: f64,
    rotation: &Quatd,
) -> Vec3d {
    let chord_length = vec::length(transform.translation);
    let start_tangent = source.normal * chord_length;
    let end_tangent = (*rotation * source.normal) * chord_length;

    let t2 = t * t;
    let t3 = t2 * t;
    start_tangent * (t3 - 2.0 * t2 + t)
        + transform.translation * (-2.0 * t3 + 3.0 * t2)
        + end_tangent * (t3 - t2)
}

/// Where the face is, part way along one repetition of the sweep.
///
/// `stationTransformWithPivot`, with `t` running from 0 at the source face to 1 at the
/// destination.
fn station_transform_with_pivot(
    source: &SweepSource,
    transform: &SweepTransform,
    parameters: &SweepParameters,
    t: f64,
    rotation_full: &Quatd,
    pivot: Option<Vec3d>,
) -> Mat4x4d {
    let c0 = source.center;
    let angle = rotation_full.angle();
    let has_rotation = angle.abs() > f64::ALMOST_ZERO;
    // The axis is degenerate at no rotation, so pick a safe default.
    let axis = if has_rotation {
        vec::normalize(rotation_full.axis())
    } else {
        Vec3d::new(0.0, 0.0, 1.0)
    };
    let rotation = Quatd::from_axis_angle(axis, angle * t);
    let one = Vec3d::new(1.0, 1.0, 1.0);
    let factors = one + (transform.scale - one) * t;

    if parameters.path_mode == SweepPathMode::Arc {
        if let Some(pivot) = pivot {
            let rise = vec::dot(transform.translation, axis);
            let mut m = mat_ext::translation_matrix(-c0);
            m = mat_ext::scaling_matrix(factors) * m;
            m = mat_ext::translation_matrix(c0) * m; // scale about c0
            m = mat_ext::translation_matrix(-pivot) * m;
            m = mat_ext::rotation_matrix_from_quat(&rotation) * m; // revolve about the pivot
            m = mat_ext::translation_matrix(pivot + axis * (rise * t)) * m; // helix lift
            return m;
        }
        // No usable pivot, fall through to the straight path.
    }

    let s_bend = parameters.path_mode == SweepPathMode::SBend
        && !vec::is_zero(source.normal, f64::ALMOST_ZERO);
    let mut m = mat_ext::translation_matrix(-c0);
    m = mat_ext::scaling_matrix(factors) * m;
    m = mat_ext::rotation_matrix_from_quat(&rotation) * m;
    m = mat_ext::translation_matrix(c0) * m;
    m = mat_ext::translation_matrix(if s_bend {
        s_bend_offset(source, transform, t, rotation_full)
    } else {
        transform.translation * t
    }) * m;
    m
}

/// The pivot an arc revolves about, or nothing for the other path modes.
fn station_arc_pivot(
    source: &SweepSource,
    transform: &SweepTransform,
    parameters: &SweepParameters,
    rotation_full: &Quatd,
) -> Option<Vec3d> {
    (parameters.path_mode == SweepPathMode::Arc)
        .then(|| arc_pivot(source, transform, rotation_full))
        .flatten()
}

/// Every station of every repetition, as the transform that puts the source face there.
///
/// `computeTransformTable`. Repetition `r` continues from where the last one ended, so it
/// applies the whole sweep's transform `r` times before the station's own.
pub fn transform_table(
    source: &SweepSource,
    transform: &SweepTransform,
    parameters: &SweepParameters,
) -> Vec<Vec<Mat4x4d>> {
    let rotation = transform.effective_rotation();
    let pivot = station_arc_pivot(source, transform, parameters, &rotation);
    let cap = station_transform_with_pivot(source, transform, parameters, 1.0, &rotation, pivot);

    // The exclusive prefix product: powers[r] is the cap transform applied r times.
    let mut powers = Vec::with_capacity(parameters.iterations);
    let mut accumulated = Mat4x4d::identity();
    for _ in 0..parameters.iterations {
        powers.push(accumulated);
        accumulated = accumulated * cap;
    }

    powers
        .into_iter()
        .map(|power| {
            (0..=parameters.segments)
                .map(|s| {
                    let t = s as f64 / parameters.segments as f64;
                    power
                        * station_transform_with_pivot(
                            source, transform, parameters, t, &rotation, pivot,
                        )
                })
                .collect()
        })
        .collect()
}

/// The brushes a sweep makes.
///
/// `generateSweepBrushes`: each face is cut into `segments` stations per repetition and
/// one brush is built from each neighbouring pair. A segment that comes out degenerate is
/// skipped rather than failing the whole sweep -- a tight arc has some.
fn generate_unchecked(
    source: &SweepSource,
    transform: &SweepTransform,
    parameters: &SweepParameters,
    material: &str,
) -> Vec<Brush> {
    if parameters.segments == 0 || parameters.iterations == 0 || source.faces.is_empty() {
        return Vec::new();
    }

    let snap_to_integer = parameters.alignment == SweepAlignment::Integer;
    let transforms = transform_table(source, transform, parameters);

    // A station vertex depends only on which station it is, so two neighbouring segments
    // compute the vertices they share identically and the mesh stays watertight even when
    // the coordinates are being rounded.
    let station = |v: Vec3d, r: usize, s: usize| {
        let p = transforms[r][s] * v;
        // Station 0 of the first repetition is the source face itself and must stay exact.
        if snap_to_integer && (s > 0 || r > 0) {
            vec::round(p)
        } else {
            p
        }
    };

    let mut result = Vec::new();
    for (face_index, face) in source.faces.iter().enumerate() {
        let mut previous: Option<Brush> = None;
        for r in 0..parameters.iterations {
            for i in 0..parameters.segments {
                let points: Vec<Vec3d> = face
                    .iter()
                    .flat_map(|v| [station(*v, r, i), station(*v, r, i + 1)])
                    .collect();

                if points.iter().any(|p| p.v.iter().any(|v| !v.is_finite())) {
                    return Vec::new();
                }
                if let Ok(geometry) = Polyhedron::from_points(&points, f64::ALMOST_ZERO) {
                    let raw = Brush::from_polyhedron(geometry, material);
                    let start: Vec<_> = face.iter().map(|v| station(*v, r, i)).collect();
                    let end: Vec<_> = face.iter().map(|v| station(*v, r, i + 1)).collect();
                    let attrs: Vec<_> = raw
                        .faces()
                        .iter()
                        .enumerate()
                        .map(|(fi, target)| {
                            let mut attr = source
                                .attributes
                                .get(face_index)
                                .cloned()
                                .unwrap_or_else(|| target.clone());
                            attr.points = target.points;
                            attr.plane = target.plane;
                            let on = |points: &Vec<Vec3d>| {
                                points.iter().all(|p| {
                                    (vec::dot(target.normal(), *p) - target.plane.distance).abs()
                                        < 1e-7
                                })
                            };
                            let cap_start = on(&start);
                            let cap_end = on(&end);
                            if (cap_start && (i > 0 || r > 0))
                                || (cap_end
                                    && (i + 1 < parameters.segments
                                        || r + 1 < parameters.iterations))
                            {
                                attr.material_name = "skip".into();
                            }
                            let polygon = raw.face_polygon(fi).unwrap_or_default();
                            let carried = previous.as_ref().and_then(|brush| {
                                brush
                                    .geometry_faces()
                                    .filter_map(|(old, vertices)| {
                                        if old.material_name.as_str() == "skip" {
                                            return None;
                                        }
                                        let shared: Vec<_> = vertices
                                            .iter()
                                            .filter(|p| {
                                                polygon
                                                    .iter()
                                                    .any(|q| vec::squared_distance(**p, *q) < 1e-12)
                                            })
                                            .collect();
                                        (shared.len() >= 2)
                                            .then(|| (old, **shared.first().unwrap()))
                                    })
                                    .max_by(|(a, _), (b, _)| {
                                        vec::dot(a.normal(), target.normal())
                                            .total_cmp(&vec::dot(b.normal(), target.normal()))
                                    })
                            });
                            if !cap_start && !cap_end {
                                if let Some((old, anchor)) = carried {
                                    attr.wrap_uv_from(old, anchor);
                                } else if let Some(source) = source.attributes.get(face_index) {
                                    let p = source.plane;
                                    let q = target.plane;
                                    let line = vec::cross(p.normal, q.normal);
                                    let denominator = vec::squared_length(line);
                                    let anchor = if denominator > 1e-12 {
                                        (vec::cross(q.normal, line) * p.distance
                                            + vec::cross(line, p.normal) * q.distance
                                            + line * vec::dot(start[0], line))
                                            / denominator
                                    } else {
                                        start[0]
                                    };
                                    attr.wrap_uv_from(source, anchor);
                                }
                            }
                            attr
                        })
                        .collect();
                    let brush = raw.with_exact_face_attributes(&attrs).unwrap();
                    previous = Some(brush.clone());
                    result.push(brush);
                }
            }
        }
    }
    result
}

/// Validated sweep: no missing cells and no solid self-intersections.
/// Counts are bounded before creating the transform table.
fn generate_checked_uncached(
    source: &SweepSource,
    transform: &SweepTransform,
    parameters: &SweepParameters,
    material: &str,
) -> Result<Vec<Brush>, String> {
    let count = parameters
        .segments
        .checked_mul(parameters.iterations)
        .and_then(|n| n.checked_mul(source.faces.len()))
        .ok_or("The sweep is too large.")?;
    if count == 0 || count > 1024 {
        return Err("Use between 1 and 1024 sweep cells in total.".into());
    }
    if source
        .faces
        .iter()
        .any(|f| f.len() < 3 || f.iter().any(|p| p.v.iter().any(|v| !v.is_finite())))
        || source
            .center
            .v
            .iter()
            .chain(source.normal.v.iter())
            .any(|v| !v.is_finite())
        || transform
            .translation
            .v
            .iter()
            .chain(transform.scale.v.iter())
            .any(|v| !v.is_finite())
        || transform.scale.v.iter().any(|v| *v <= 0.0)
        || !transform.effective_rotation().angle().is_finite()
    {
        return Err("The sweep needs finite vertices, a finite turn and positive scale.".into());
    }
    let mut result = generate_unchecked(source, transform, parameters, material);
    if result.len() != count {
        return Err("A sweep segment collapsed. Reduce the turn, disable integer snapping or increase its size.".into());
    }
    let bounds: Vec<_> = result.iter().map(Brush::bounds).collect();
    for i in 0..result.len() {
        for j in i + 1..result.len() {
            if (0..3).any(|axis| {
                bounds[i].max[axis].min(bounds[j].max[axis])
                    - bounds[i].min[axis].max(bounds[j].min[axis])
                    <= 1e-6
            }) {
                continue;
            }
            if map_model::csg::intersect(&result[i], &result[j]).is_some_and(|b| {
                b.volume() > result[i].volume().min(result[j].volume()).max(1.0) * 1e-7
            }) {
                return Err(format!("Sweep cells {} and {} intersect. Increase the bend radius or reduce the profile/turn.",i+1,j+1));
            }
        }
    }
    // Only full, opposed polygon contacts are hidden. Exposed caps remain visible.
    let mut contacts = std::collections::BTreeMap::<Vec<[i64; 3]>, Vec<(usize, usize)>>::new();
    for (bi, brush) in result.iter().enumerate() {
        for (fi, _) in brush.faces().iter().enumerate() {
            let Some(polygon) = brush.face_polygon(fi) else {
                continue;
            };
            let mut key: Vec<_> = polygon
                .iter()
                .map(|p| {
                    [
                        (p[0] * 1e5).round() as i64,
                        (p[1] * 1e5).round() as i64,
                        (p[2] * 1e5).round() as i64,
                    ]
                })
                .collect();
            key.sort();
            contacts.entry(key).or_default().push((bi, fi));
        }
    }
    let mut attrs: Vec<_> = result.iter().map(|b| b.faces().to_vec()).collect();
    for contact in contacts.values() {
        if let [(a, fa), (b, fb)] = contact.as_slice() {
            if vec::dot(attrs[*a][*fa].normal(), attrs[*b][*fb].normal()) < -1.0 + 1e-6
                && (attrs[*a][*fa].plane.distance + attrs[*b][*fb].plane.distance).abs() < 1e-7
                && result[*a].face_polygon(*fa).unwrap().iter().all(|p| {
                    result[*b]
                        .face_polygon(*fb)
                        .unwrap()
                        .iter()
                        .any(|q| vec::squared_distance(*p, *q) < 1e-14)
                })
            {
                attrs[*a][*fa].material_name = "skip".into();
                attrs[*b][*fb].material_name = "skip".into();
            }
        }
    }
    for (brush, faces) in result.iter_mut().zip(attrs) {
        *brush = brush.with_exact_face_attributes(&faces).unwrap();
    }
    Ok(result)
}

type CachedSweep = (
    SweepSource,
    SweepTransform,
    SweepParameters,
    String,
    Result<Vec<Brush>, String>,
);
thread_local! {static SWEEP_CACHE:std::cell::RefCell<Option<CachedSweep>>=const{std::cell::RefCell::new(None)};}

/// Shares validated geometry between the inspector and multiple viewports. Returned
/// brushes are independent copies; all source attributes participate in invalidation.
pub fn generate_checked(
    source: &SweepSource,
    transform: &SweepTransform,
    parameters: &SweepParameters,
    material: &str,
) -> Result<Vec<Brush>, String> {
    SWEEP_CACHE.with(|cache| {
        if let Some((old_source, old_transform, old_parameters, old_material, result)) =
            cache.borrow().as_ref()
        {
            if old_source == source
                && old_transform == transform
                && old_parameters == parameters
                && old_material == material
            {
                return result.clone();
            }
        }
        let result = generate_checked_uncached(source, transform, parameters, material);
        *cache.borrow_mut() = Some((
            source.clone(),
            *transform,
            *parameters,
            material.to_owned(),
            result.clone(),
        ));
        result
    })
}

/// Preview compatibility: a refused sweep displays no partial geometry.
pub fn generate_brushes(
    source: &SweepSource,
    transform: &SweepTransform,
    parameters: &SweepParameters,
    material: &str,
) -> Vec<Brush> {
    generate_checked(source, transform, parameters, material).unwrap_or_default()
}

/// The bounds the swept brushes would occupy, for framing or for checking they fit.
pub fn bounds_of(brushes: &[Brush]) -> Option<Bbox3d> {
    let mut bounds: Option<Bbox3d> = None;
    for brush in brushes {
        bounds = Some(match bounds {
            Some(bounds) => bounds.merged(&brush.bounds()),
            None => brush.bounds(),
        });
    }
    bounds
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 64-unit square facing up, at the origin.
    fn square() -> SweepSource {
        SweepSource {
            attributes: Vec::new(),
            faces: vec![vec![
                Vec3d::new(-32.0, -32.0, 0.0),
                Vec3d::new(32.0, -32.0, 0.0),
                Vec3d::new(32.0, 32.0, 0.0),
                Vec3d::new(-32.0, 32.0, 0.0),
            ]],
            center: Vec3d::zero(),
            normal: Vec3d::new(0.0, 0.0, 1.0),
        }
    }

    fn straight_up(distance: f64) -> SweepTransform {
        SweepTransform {
            translation: Vec3d::new(0.0, 0.0, distance),
            ..Default::default()
        }
    }

    fn straight(segments: usize, iterations: usize) -> SweepParameters {
        SweepParameters {
            segments,
            iterations,
            path_mode: SweepPathMode::Straight,
            alignment: SweepAlignment::Free,
        }
    }

    #[test]
    fn a_transform_that_does_nothing_is_recognised() {
        assert!(SweepTransform::default().is_no_op());
        assert!(!straight_up(64.0).is_no_op());
        assert!(!SweepTransform {
            scale: Vec3d::new(2.0, 1.0, 1.0),
            ..Default::default()
        }
        .is_no_op());
    }

    #[test]
    fn a_turn_past_a_half_is_taken_the_short_way_round() {
        // 330 degrees one way is 30 degrees the other, and sweeping all 330 is not what
        // dragging a ring that far meant.
        let long_way = SweepTransform {
            rotation: Quatd::from_axis_angle(Vec3d::new(0.0, 0.0, 1.0), 330.0f64.to_radians()),
            ..Default::default()
        };
        let effective = long_way.effective_rotation();
        geometry_math::assert_approx_eq!(effective.angle().to_degrees(), 30.0, 1e-6);
        // About the opposite axis, so it turns the other way.
        geometry_math::assert_approx_eq!(
            vec::normalize(effective.axis()),
            Vec3d::new(0.0, 0.0, -1.0),
            1e-6
        );
    }

    #[test]
    fn a_turn_under_a_half_is_left_alone() {
        let short_way = SweepTransform {
            rotation: Quatd::from_axis_angle(Vec3d::new(0.0, 0.0, 1.0), 45.0f64.to_radians()),
            ..Default::default()
        };
        geometry_math::assert_approx_eq!(
            short_way.effective_rotation().angle().to_degrees(),
            45.0,
            1e-6
        );
    }

    #[test]
    fn a_straight_sweep_makes_one_brush_per_segment() {
        let brushes = generate_brushes(&square(), &straight_up(64.0), &straight(4, 1), "stone");
        assert_eq!(brushes.len(), 4);

        // Together they fill the whole travel.
        let bounds = bounds_of(&brushes).expect("some brushes");
        geometry_math::assert_approx_eq!(bounds.min.z(), 0.0, 1e-6);
        geometry_math::assert_approx_eq!(bounds.max.z(), 64.0, 1e-6);
        // And each is a box the width of the face.
        for brush in &brushes {
            brush.geometry().check_invariant().expect("a valid mesh");
            geometry_math::assert_approx_eq!(brush.bounds().size().x(), 64.0, 1e-6);
        }
    }

    #[test]
    fn repeating_a_sweep_carries_on_from_where_it_ended() {
        let brushes = generate_brushes(&square(), &straight_up(64.0), &straight(2, 3), "stone");
        assert_eq!(brushes.len(), 6, "two segments, three times");

        let bounds = bounds_of(&brushes).expect("some brushes");
        geometry_math::assert_approx_eq!(bounds.max.z(), 192.0, 1e-6);
    }

    #[test]
    fn a_sweep_that_goes_nowhere_makes_nothing_usable() {
        // Every station is the same, so every segment is flat.
        let brushes = generate_brushes(
            &square(),
            &SweepTransform::default(),
            &straight(4, 1),
            "stone",
        );
        assert!(brushes.is_empty());
    }

    #[test]
    fn scaling_along_the_sweep_makes_a_taper() {
        let transform = SweepTransform {
            translation: Vec3d::new(0.0, 0.0, 64.0),
            scale: Vec3d::new(0.5, 0.5, 1.0),
            ..Default::default()
        };
        let brushes = generate_brushes(&square(), &transform, &straight(4, 1), "stone");
        assert_eq!(brushes.len(), 4);

        // The last segment is narrower than the first.
        let first = brushes[0].bounds().size().x();
        let last = brushes[3].bounds().size().x();
        assert!(last < first, "{last} is not narrower than {first}");
    }

    #[test]
    fn an_arc_bends_the_sweep_round_a_pivot() {
        // A quarter turn about Y, travelling along X: the face should come round rather
        // than going straight.
        let transform = SweepTransform {
            translation: Vec3d::new(128.0, 0.0, 0.0),
            rotation: Quatd::from_axis_angle(Vec3d::new(0.0, 1.0, 0.0), 90.0f64.to_radians()),
            ..Default::default()
        };
        let parameters = SweepParameters {
            segments: 8,
            iterations: 1,
            path_mode: SweepPathMode::Arc,
            alignment: SweepAlignment::Free,
        };

        // There is a circle that does both, so the arc has a pivot.
        let rotation = transform.effective_rotation();
        assert!(arc_pivot(&square(), &transform, &rotation).is_some());

        let brushes = generate_brushes(&square(), &transform, &parameters, "stone");
        assert_eq!(brushes.len(), 8);

        // The middle of the arc lifts away from the straight line between the ends.
        let table = transform_table(&square(), &transform, &parameters);
        let middle = table[0][parameters.segments / 2] * Vec3d::zero();
        let straight_middle = Vec3d::new(64.0, 0.0, 0.0);
        assert!(
            vec::distance(middle, straight_middle) > 1.0,
            "the arc did not bend: {middle:?}"
        );
    }

    #[test]
    fn an_arc_with_no_turn_falls_back_to_a_straight_line() {
        // With no rotation, no circle can be fitted; the sweep uses the remaining transform.
        let parameters = SweepParameters {
            segments: 4,
            iterations: 1,
            path_mode: SweepPathMode::Arc,
            alignment: SweepAlignment::Free,
        };
        let transform = straight_up(64.0);
        assert!(arc_pivot(&square(), &transform, &transform.effective_rotation()).is_none());

        let brushes = generate_brushes(&square(), &transform, &parameters, "stone");
        assert_eq!(brushes.len(), 4);
        let bounds = bounds_of(&brushes).expect("some brushes");
        geometry_math::assert_approx_eq!(bounds.max.z(), 64.0, 1e-6);
    }

    #[test]
    fn an_s_bend_leaves_along_the_face_and_arrives_along_the_turned_one() {
        let transform = SweepTransform {
            translation: Vec3d::new(128.0, 0.0, 0.0),
            ..Default::default()
        };
        let parameters = SweepParameters {
            segments: 8,
            iterations: 1,
            path_mode: SweepPathMode::SBend,
            alignment: SweepAlignment::Free,
        };

        // The face points up and travels along X, so the curve leaves upwards before it
        // comes back: a straight line would not.
        let table = transform_table(&square(), &transform, &parameters);
        let quarter = table[0][parameters.segments / 4] * Vec3d::zero();
        assert!(
            quarter.z() > 1.0,
            "the S-bend did not leave upwards: {quarter:?}"
        );

        // And it still arrives where it was asked to.
        let end = table[0][parameters.segments] * Vec3d::zero();
        geometry_math::assert_approx_eq!(end, Vec3d::new(128.0, 0.0, 0.0), 1e-6);
    }

    #[test]
    fn integer_alignment_puts_the_stations_on_whole_coordinates() {
        let transform = straight_up(65.0);
        let parameters = SweepParameters {
            segments: 3,
            iterations: 1,
            path_mode: SweepPathMode::Straight,
            alignment: SweepAlignment::Integer,
        };

        let brushes = generate_brushes(&square(), &transform, &parameters, "stone");
        assert_eq!(brushes.len(), 3);

        // The first station is the source face and stays exact; every other corner is on a
        // whole coordinate.
        for brush in &brushes {
            for vertex in brush.vertex_positions() {
                let rounded = vec::round(vertex);
                assert!(
                    vec::distance(vertex, rounded) < 0.01,
                    "{vertex:?} is not on a whole coordinate"
                );
            }
        }
    }

    #[test]
    fn neighbouring_segments_share_their_corners_exactly() {
        // This is what keeps a rounded sweep watertight: the station a segment ends at and
        // the one the next begins at are computed the same way, so they agree exactly.
        let transform = straight_up(65.0);
        let parameters = SweepParameters {
            segments: 4,
            iterations: 1,
            path_mode: SweepPathMode::Straight,
            alignment: SweepAlignment::Integer,
        };
        let table = transform_table(&square(), &transform, &parameters);

        for s in 0..parameters.segments {
            let point = Vec3d::new(-32.0, -32.0, 0.0);
            let end_of_this = vec::round(table[0][s + 1] * point);
            let start_of_next = vec::round(table[0][s + 1] * point);
            assert_eq!(end_of_this, start_of_next);
        }
    }

    #[test]
    fn a_sweep_of_nothing_makes_nothing() {
        let empty = SweepSource {
            attributes: Vec::new(),
            faces: Vec::new(),
            center: Vec3d::zero(),
            normal: Vec3d::new(0.0, 0.0, 1.0),
        };
        assert!(generate_brushes(&empty, &straight_up(64.0), &straight(4, 1), "stone").is_empty());

        // And neither does a sweep with no segments to cut it into.
        assert!(
            generate_brushes(&square(), &straight_up(64.0), &straight(0, 1), "stone").is_empty()
        );
    }
}

#[cfg(test)]
mod upgrade_tests {
    use super::*;
    fn source() -> SweepSource {
        let points = vec![
            Vec3d::new(-8.0, -8.0, 0.0),
            Vec3d::new(8.0, -8.0, 0.0),
            Vec3d::new(8.0, 8.0, 0.0),
            Vec3d::new(-8.0, 8.0, 0.0),
        ];
        let mut face = map_model::brush_face::BrushFace::from_points(
            [points[0], points[1], points[2]],
            "tile",
            map_model::uv::UvCoordSystem::Paraxial,
            map_model::uv::UvAttributes::default(),
        )
        .unwrap();
        face.set_paint(Some(map_model::brush_face::FaceColor::Solid {
            color: map_model::brush_face::Rgb::new(10, 80, 140),
            alpha: 255,
        }));
        face.surface_material = Some(map_model::brush_face::SurfaceMaterial {
            roughness: 35,
            metallic: 80,
            emission: 4,
        });
        SweepSource {
            faces: vec![points],
            attributes: vec![face],
            center: Vec3d::zero(),
            normal: Vec3d::new(0.0, 0.0, 1.0),
        }
    }
    #[test]
    fn checked_sweep_preserves_surface_and_uv_across_stations() {
        let source = source();
        for curved in [false, true] {
            let transform = if curved {
                SweepTransform {
                    translation: Vec3d::new(64.0, 0.0, 64.0),
                    rotation: Quatd::from_axis_angle(
                        Vec3d::new(0.0, 1.0, 0.0),
                        std::f64::consts::FRAC_PI_2,
                    ),
                    ..Default::default()
                }
            } else {
                SweepTransform {
                    translation: Vec3d::new(0.0, 0.0, 64.0),
                    ..Default::default()
                }
            };
            let parameters = SweepParameters {
                segments: 8,
                iterations: 1,
                alignment: SweepAlignment::Free,
                path_mode: if curved {
                    SweepPathMode::Arc
                } else {
                    SweepPathMode::Straight
                },
            };
            let brushes = generate_checked(&source, &transform, &parameters, "fallback").unwrap();
            assert_eq!(brushes.len(), 8);
            assert_eq!(
                brushes
                    .iter()
                    .flat_map(|b| b.faces())
                    .filter(|f| f.material_name.as_str() == "skip")
                    .count(),
                14
            );
            for face in brushes
                .iter()
                .flat_map(|b| b.faces())
                .filter(|f| f.material_name.as_str() != "skip")
            {
                assert_eq!(face.material_name.as_str(), "tile");
                assert_eq!(face.paint(), source.attributes[0].paint());
                assert_eq!(face.surface_material, source.attributes[0].surface_material);
            }
            let mut checks = 0;
            for pair in brushes.windows(2) {
                for (a, pa) in pair[0]
                    .geometry_faces()
                    .filter(|(f, _)| f.material_name.as_str() != "skip")
                {
                    for (b, pb) in pair[1]
                        .geometry_faces()
                        .filter(|(f, _)| f.material_name.as_str() != "skip")
                    {
                        let shared: Vec<_> = pa
                            .iter()
                            .filter(|p| pb.iter().any(|q| vec::squared_distance(**p, *q) < 1e-12))
                            .collect();
                        if shared.len() < 2 {
                            continue;
                        }
                        for p in shared {
                            let delta = a.uv_coords(*p) - b.uv_coords(*p);
                            assert!(
                                delta.x().abs() < 0.001 && delta.y().abs() < 0.001,
                                "sweep UV seam {delta:?}"
                            );
                        }
                        checks += 1;
                    }
                }
            }
            assert_eq!(checks, 28);
        }
    }
    #[test]
    fn checked_sweep_refuses_collapsed_excessive_and_looping_paths() {
        let source = source();
        let mut parameters = SweepParameters {
            segments: 8,
            iterations: 1,
            alignment: SweepAlignment::Integer,
            path_mode: SweepPathMode::Straight,
        };
        let tiny = SweepTransform {
            translation: Vec3d::new(0.0, 0.0, 0.01),
            ..Default::default()
        };
        assert!(generate_checked(&source, &tiny, &parameters, "stone").is_err());
        parameters.iterations = usize::MAX;
        assert!(generate_checked(&source, &tiny, &parameters, "stone").is_err());
        parameters.iterations = 5;
        parameters.alignment = SweepAlignment::Free;
        parameters.path_mode = SweepPathMode::Arc;
        let looped = SweepTransform {
            translation: Vec3d::new(64.0, 0.0, 64.0),
            rotation: Quatd::from_axis_angle(
                Vec3d::new(0.0, 1.0, 0.0),
                std::f64::consts::FRAC_PI_2,
            ),
            ..Default::default()
        };
        assert!(generate_checked(&source, &looped, &parameters, "stone")
            .unwrap_err()
            .contains("intersect"));
    }
}

#[cfg(test)]
mod cache_upgrade_tests {
    use super::*;
    #[test]
    fn sweep_cache_tracks_surface_changes_and_returns_independent_geometry() {
        let points = vec![
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(8.0, 0.0, 0.0),
            Vec3d::new(8.0, 8.0, 0.0),
            Vec3d::new(0.0, 8.0, 0.0),
        ];
        let face = map_model::brush_face::BrushFace::from_points(
            [points[0], points[1], points[2]],
            "old",
            map_model::uv::UvCoordSystem::Paraxial,
            map_model::uv::UvAttributes::default(),
        )
        .unwrap();
        let mut source = SweepSource {
            faces: vec![points],
            attributes: vec![face],
            center: Vec3d::new(4.0, 4.0, 0.0),
            normal: Vec3d::new(0.0, 0.0, 1.0),
        };
        let transform = SweepTransform {
            translation: Vec3d::new(0.0, 0.0, 32.0),
            ..Default::default()
        };
        let parameters = SweepParameters {
            segments: 4,
            iterations: 1,
            path_mode: SweepPathMode::Straight,
            alignment: SweepAlignment::Free,
        };
        let mut first = generate_checked(&source, &transform, &parameters, "fallback").unwrap();
        let mut changed = first[0].faces().to_vec();
        changed[0].material_name = "modified".into();
        first[0] = first[0].with_exact_face_attributes(&changed).unwrap();
        let second = generate_checked(&source, &transform, &parameters, "fallback").unwrap();
        assert!(second
            .iter()
            .flat_map(|b| b.faces())
            .all(|f| f.material_name.as_str() != "modified"));
        source.attributes[0].material_name = "updated".into();
        let updated = generate_checked(&source, &transform, &parameters, "fallback").unwrap();
        assert!(updated
            .iter()
            .flat_map(|b| b.faces())
            .filter(|f| f.material_name.as_str() != "skip")
            .all(|f| f.material_name.as_str() == "updated"));
    }
}
