//! Generators for the shapes the editor can insert.
//!
//! Everything here produces convex solids inscribed in a bounding box, so the same drag
//! that sizes a box sizes any of them. A brush must be convex, so shapes that are not —
//! a hollow cylinder, an arch — come back as several brushes rather than one.
//!
//! The shapes are built from their vertices and hulled, not from tangent planes. That is
//! what keeps a cylinder's vertices exactly on the box the user dragged instead of poking
//! out past its corners.

use geometry_math::bbox::{Bbox2d, Bbox3d};
use geometry_math::line::Line2d;
use geometry_math::mat::{Mat3x3d, Mat4x4d};
use geometry_math::mat_ext::{
    rotation_matrix_from_to, scaling_matrix, scaling_matrix_2d, translation_matrix,
    translation_matrix_2d,
};
use geometry_math::scalar::Float;
use geometry_math::util::Axis;
use geometry_math::vec::{self, Vec2d, Vec3d};

use crate::brush::{Brush, BrushError};
use crate::brush_face::BrushFace;
use crate::polyhedron::Polyhedron;
use crate::uv::{UvAttributes, UvCoordSystem};

/// Why a shape could not be built.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeError {
    /// Fewer sides than a solid can have.
    TooFewSides {
        /// How many were asked for.
        given: usize,
    },
    /// The bounding box has no extent on some axis, so the shape would be flat.
    FlatBounds,
    /// The planes did not make a solid.
    Invalid(BrushError),
    /// The points did not make a solid.
    Hull(crate::polyhedron::HullError),
    /// Two sides of a hollow shape run parallel, so its wall has no inside corner.
    NoLineIntersection,
    /// A bounded generator option is invalid.
    InvalidParameter(&'static str),
}

impl std::fmt::Display for ShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShapeError::TooFewSides { given } => {
                write!(f, "a shape needs at least 3 sides, got {given}")
            }
            ShapeError::FlatBounds => {
                f.write_str("the bounding box is flat, so the shape would enclose nothing")
            }
            ShapeError::Invalid(cause) => write!(f, "{cause}"),
            ShapeError::Hull(cause) => write!(f, "{cause}"),
            ShapeError::InvalidParameter(message) => f.write_str(message),
            ShapeError::NoLineIntersection => f.write_str("failed to intersect lines"),
        }
    }
}

impl std::error::Error for ShapeError {}

/// The default material a newly inserted shape gets.
pub const DEFAULT_MATERIAL: &str = "__TB_empty";

/// How the vertices of a circular shape are placed.
///
/// `map_model::CircleShape`. The distinction matters because a circle drawn inside a box can
/// either touch the box at its vertices or at the middle of its edges, and which one is
/// wanted depends on what is being lined up against what.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleShape {
    /// The middle of each edge touches the box, so a cylinder shares the box's footprint.
    EdgeAligned {
        /// How many sides go around.
        num_sides: usize,
    },
    /// Each vertex touches the box.
    VertexAligned {
        /// How many sides go around.
        num_sides: usize,
    },
    /// A circle that stays a circle when scaled, refined `precision` times.
    Scalable {
        /// How many times to clip the corners; each pass doubles the vertex count.
        precision: usize,
    },
}

impl Default for CircleShape {
    fn default() -> Self {
        CircleShape::EdgeAligned { num_sides: 8 }
    }
}

impl CircleShape {
    /// Returns how many vertices go around the circle.
    pub fn vertex_count(self) -> usize {
        match self {
            CircleShape::EdgeAligned { num_sides } | CircleShape::VertexAligned { num_sides } => {
                num_sides
            }
            CircleShape::Scalable { precision } => 12 * (1 << precision),
        }
    }

    /// Returns the error for a shape that cannot be built, if there is one.
    fn check(self) -> Result<(), ShapeError> {
        match self {
            CircleShape::EdgeAligned { num_sides } | CircleShape::VertexAligned { num_sides }
                if num_sides < 3 =>
            {
                Err(ShapeError::TooFewSides { given: num_sides })
            }
            _ => Ok(()),
        }
    }
}

/* ========== circles ========== */

/// The transform taking a circle on `[-1, 1]` to one filling `bounds`.
fn circle_transform(bounds: &Bbox2d) -> Mat3x3d {
    translation_matrix_2d(bounds.min)
        * scaling_matrix_2d(bounds.size())
        * translation_matrix_2d(Vec2d::new(0.5, 0.5))
        * scaling_matrix_2d(Vec2d::new(0.5, 0.5))
}

/// A circle whose edges touch the bounds at their midpoints.
fn make_edge_aligned_circle(num_sides: usize, bounds: &Bbox2d) -> Vec<Vec2d> {
    debug_assert!(num_sides > 2);

    let transform = circle_transform(bounds);
    let n = num_sides as f64;

    (0..num_sides)
        .map(|i| {
            let angle = (i as f64 + 0.5) * std::f64::consts::TAU / n - std::f64::consts::FRAC_PI_2;
            // Half the angle one side subtends. Dividing by its cosine pushes the
            // vertices out far enough that the edges, not the vertices, touch the bounds.
            let a = std::f64::consts::PI / n;
            let ca = a.cos();
            transform * Vec2d::new(angle.cos() / ca, angle.sin() / ca)
        })
        .collect()
}

/// A circle whose vertices touch the bounds.
fn make_vertex_aligned_circle(num_sides: usize, bounds: &Bbox2d) -> Vec<Vec2d> {
    debug_assert!(num_sides > 2);

    let transform = circle_transform(bounds);
    let n = num_sides as f64;

    (0..num_sides)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::TAU / n - std::f64::consts::FRAC_PI_2;
            transform * Vec2d::new(angle.cos(), angle.sin())
        })
        .collect()
}

/// A circle that survives being scaled, refined by clipping its corners.
///
/// Stretching this one moves the right half and the top half apart rather than scaling
/// every vertex, so the rounded corners keep their radius instead of turning into
/// ellipse quadrants.
fn make_scalable_circle(precision: usize, bounds: &Bbox2d) -> Vec<Vec2d> {
    let mut vertices = vec![
        Vec2d::new(-0.25, 1.00),
        Vec2d::new(-0.75, 0.75),
        Vec2d::new(-1.00, 0.25),
        Vec2d::new(-1.00, -0.25),
        Vec2d::new(-0.75, -0.75),
        Vec2d::new(-0.25, -1.00),
        Vec2d::new(0.25, -1.00),
        Vec2d::new(0.75, -0.75),
        Vec2d::new(1.00, -0.25),
        Vec2d::new(1.00, 0.25),
        Vec2d::new(0.75, 0.75),
        Vec2d::new(0.25, 1.00),
    ];

    // Clip off each corner to get a scalable unit circle with double the vertices.
    for _ in 0..precision {
        let previous = std::mem::take(&mut vertices);
        let count = previous.len();
        for j in 0..count {
            let prev = previous[(j + count - 1) % count];
            let cur = previous[j];
            let next = previous[(j + 1) % count];

            vertices.push(prev + (cur - prev) * 0.75);
            vertices.push(cur + (next - cur) * 0.25);
        }
    }

    let size = bounds.size();
    let min_size = size.x().min(size.y());
    let square_size = Vec2d::new(min_size, min_size);

    let transform = scaling_matrix_2d(square_size)
        * translation_matrix_2d(Vec2d::new(0.5, 0.5))
        * scaling_matrix_2d(Vec2d::new(0.5, 0.5));
    for vertex in &mut vertices {
        *vertex = transform * *vertex;
    }

    // Stretch the circle to fit the bounds by moving the right half and the top half
    // instead of uniformly scaling all vertices.
    let offset = Vec2d::new(
        (size.x() - size.y()).max(0.0),
        (size.y() - size.x()).max(0.0),
    );

    for v in &mut vertices {
        if v.x() > min_size / 2.0 {
            *v = Vec2d::new(v.x() + offset.x(), v.y());
        }
        if v.y() > min_size / 2.0 {
            *v = Vec2d::new(v.x(), v.y() + offset.y());
        }
    }

    let to_bounds = translation_matrix_2d(bounds.min);
    vertices.iter().map(|v| to_bounds * *v).collect()
}

/// The circle a shape's cross-section follows.
fn make_circle(circle_shape: CircleShape, bounds: &Bbox2d) -> Vec<Vec2d> {
    match circle_shape {
        CircleShape::EdgeAligned { num_sides } => make_edge_aligned_circle(num_sides, bounds),
        CircleShape::VertexAligned { num_sides } => make_vertex_aligned_circle(num_sides, bounds),
        CircleShape::Scalable { precision } => make_scalable_circle(precision, bounds),
    }
}

/* ========== solids of revolution ========== */

/// The footprint of a box, as 2D bounds.
fn footprint(bounds: &Bbox3d) -> Bbox2d {
    Bbox2d::new(bounds.min.xy(), bounds.max.xy())
}

/// Lifts 2D points to a given height.
fn set_z(vertices: &[Vec2d], z: f64) -> Vec<Vec3d> {
    vertices
        .iter()
        .map(|v| Vec3d::new(v.x(), v.y(), z))
        .collect()
}

fn make_cylinder(circle_shape: CircleShape, bounds_xy: &Bbox3d) -> Vec<Vec3d> {
    let mut vertices = Vec::new();
    for v in make_circle(circle_shape, &footprint(bounds_xy)) {
        vertices.push(Vec3d::new(v.x(), v.y(), bounds_xy.min.z()));
        vertices.push(Vec3d::new(v.x(), v.y(), bounds_xy.max.z()));
    }
    vertices
}

/// If a scalable cone is stretched, it doesn't have one vertex as the tip. Instead, the
/// tip is an edge.
fn make_scalable_cone_tip(bounds_xy: &Bbox3d) -> Vec<Vec2d> {
    let xy = footprint(bounds_xy);
    let offset = xy.size().x().min(xy.size().y()) / 2.0;
    let mut tip = vec![
        Vec2d::new(xy.min.x() + offset, xy.min.y() + offset),
        Vec2d::new(xy.min.x() + offset, xy.max.y() - offset),
        Vec2d::new(xy.max.x() - offset, xy.min.y() + offset),
        Vec2d::new(xy.max.x() - offset, xy.max.y() - offset),
    ];
    // `kdl::vec_sort_and_remove_duplicates`: a square footprint collapses all four to one
    // point, and an oblong one to a pair.
    tip.sort_by(|a, b| {
        (a.x(), a.y())
            .partial_cmp(&(b.x(), b.y()))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    tip.dedup();
    tip
}

fn make_cone(circle_shape: CircleShape, bounds_xy: &Bbox3d) -> Vec<Vec3d> {
    match circle_shape {
        CircleShape::Scalable { precision } => {
            let mut vertices = set_z(
                &make_scalable_circle(precision, &footprint(bounds_xy)),
                bounds_xy.min.z(),
            );
            vertices.extend(set_z(&make_scalable_cone_tip(bounds_xy), bounds_xy.max.z()));
            vertices
        }
        _ => {
            let mut vertices = set_z(
                &make_circle(circle_shape, &footprint(bounds_xy)),
                bounds_xy.min.z(),
            );
            let center = footprint(bounds_xy).center();
            vertices.push(Vec3d::new(center.x(), center.y(), bounds_xy.max.z()));
            vertices
        }
    }
}

/* ========== hollow shapes ========== */

/// The corners a hollow shape's wedges meet at when the bounds are too small for an inner
/// circle.
///
/// If the bounds are square, all corners coincide. If the bounds are rectangular, two
/// pairs of corners coincide. Each vertex of the outer circle then maps to the corner
/// nearest it.
fn make_vertices_for_wedges(outer_circle: &[Vec2d], bounds: &Bbox2d) -> Vec<Vec2d> {
    let offset = bounds.size().x().min(bounds.size().y()) / 2.0;
    let corners = [
        Vec2d::new(bounds.min.x() + offset, bounds.min.y() + offset),
        Vec2d::new(bounds.min.x() + offset, bounds.max.y() - offset),
        Vec2d::new(bounds.max.x() - offset, bounds.min.y() + offset),
        Vec2d::new(bounds.max.x() - offset, bounds.max.y() - offset),
    ];

    outer_circle
        .iter()
        .map(|v| {
            *corners
                .iter()
                .min_by(|a, b| {
                    vec::squared_distance(*v, **a)
                        .partial_cmp(&vec::squared_distance(*v, **b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .expect("there are four corners")
        })
        .collect()
}

/// The inside of a hollow shape's wall, `thickness` in from the outside.
fn make_hollow_cylinder_inner_circle(
    outer_circle: &[Vec2d],
    thickness: f64,
    circle_shape: CircleShape,
    bounds: &Bbox2d,
) -> Result<Vec<Vec2d>, ShapeError> {
    if bounds.size().x() <= thickness * 2.0 || bounds.size().y() <= thickness * 2.0 {
        return Ok(make_vertices_for_wedges(outer_circle, bounds));
    }

    match circle_shape {
        CircleShape::Scalable { precision } => {
            let delta = Vec2d::new(thickness, thickness);
            let inner_bounds = Bbox2d::new(bounds.min + delta, bounds.max - delta);
            Ok(make_scalable_circle(precision, &inner_bounds))
        }
        _ => {
            // Offset every side inwards by the wall thickness, then take the corners of
            // the offset sides. Insetting the vertices instead would thin the wall at
            // every corner.
            let num_sides = outer_circle.len();
            let outer_lines: Vec<Line2d> = (0..num_sides)
                .map(|i| {
                    let p1 = outer_circle[i];
                    let p2 = outer_circle[(i + 1) % num_sides];
                    Line2d {
                        point: p1,
                        direction: vec::normalize(p2 - p1),
                    }
                })
                .collect();

            let inner_lines: Vec<Line2d> = outer_lines
                .iter()
                .map(|l| {
                    let offset_dir = Vec2d::new(-l.direction.y(), l.direction.x());
                    Line2d {
                        point: l.point + offset_dir * thickness,
                        direction: l.direction,
                    }
                })
                .collect();

            (0..num_sides)
                .map(|i| {
                    let l1 = &inner_lines[(i + num_sides - 1) % num_sides];
                    let l2 = &inner_lines[i];
                    let d = geometry_math::intersection::intersect_line_line(l1, l2)
                        .ok_or(ShapeError::NoLineIntersection)?;
                    Ok(l1.point + l1.direction * d)
                })
                .collect()
        }
    }
}

/// The eight corners of one segment of a hollow cylinder's wall.
fn make_hollow_cylinder_fragment_vertices(
    outer_circle: &[Vec2d],
    inner_circle: &[Vec2d],
    i: usize,
    bounds_xy: &Bbox3d,
) -> Vec<Vec3d> {
    debug_assert_eq!(outer_circle.len(), inner_circle.len());

    let num_sides = outer_circle.len();
    let po = outer_circle[i % num_sides];
    let pi = inner_circle[i % num_sides];
    let no = outer_circle[(i + 1) % num_sides];
    let ni = inner_circle[(i + 1) % num_sides];

    let (low, high) = (bounds_xy.min.z(), bounds_xy.max.z());
    [po, pi, no, ni]
        .into_iter()
        .flat_map(|p| {
            [
                Vec3d::new(p.x(), p.y(), low),
                Vec3d::new(p.x(), p.y(), high),
            ]
        })
        .collect()
}

/* ========== arches ========== */

/// Maps the tunnel (extrusion) axis to the span and vertical axes of the arch's
/// cross-section. Arches rise along world Z where possible so they stand upright.
#[derive(Debug, Clone, Copy)]
struct ArchAxes {
    tunnel: usize,
    span: usize,
    vertical: usize,
}

fn arch_axes(axis: Axis) -> ArchAxes {
    match axis {
        Axis::X => ArchAxes {
            tunnel: 0,
            span: 1,
            vertical: 2,
        },
        Axis::Y => ArchAxes {
            tunnel: 1,
            span: 0,
            vertical: 2,
        },
        Axis::Z => ArchAxes {
            tunnel: 2,
            span: 0,
            vertical: 1,
        },
    }
}

/// Interpolates the point on segment a->b at vertical coordinate v.
fn crossing_at_v(a: Vec2d, b: Vec2d, v: f64) -> Vec2d {
    let t = (v - a.y()) / (b.y() - a.y());
    Vec2d::new(a.x() + (b.x() - a.x()) * t, v)
}

/// Returns the normal of segment a->b that points away from the given center.
fn outward_normal(a: Vec2d, b: Vec2d, center: Vec2d) -> Vec2d {
    let d = b - a;
    let normal = Vec2d::new(-d.y(), d.x());
    if vec::dot(normal, (a + b) / 2.0 - center) < 0.0 {
        -normal
    } else {
        normal
    }
}

/// Indices of the contiguous run of circle vertices at or above the springing line.
fn arch_upper_run(circle: &[Vec2d], v_min: f64) -> Vec<usize> {
    let n = circle.len();
    let is_upper = |i: usize| circle[i].y() >= v_min;

    // Start of the contiguous upper run: an upper vertex whose predecessor is below.
    let first_upper = (0..n).find(|&i| is_upper(i) && !is_upper((i + n - 1) % n));

    match first_upper {
        None => Vec::new(),
        Some(first) => (0..n)
            .map(|i| (first + i) % n)
            .take_while(|&i| is_upper(i))
            .collect(),
    }
}

/// Walks the given run along the given circle, capping both ends with a foot on the
/// springing line so that the arch sits flat.
fn arch_boundary(circle: &[Vec2d], upper: &[usize], v_min: f64) -> Vec<Vec2d> {
    let n = circle.len();
    let first = upper[0];
    let last = upper[upper.len() - 1];
    let before_first = (first + n - 1) % n;
    let after_last = (last + 1) % n;

    let mut boundary = Vec::with_capacity(upper.len() + 2);
    boundary.push(crossing_at_v(circle[before_first], circle[first], v_min));
    for &i in upper {
        boundary.push(Vec2d::new(circle[i].x(), circle[i].y().max(v_min)));
    }
    boundary.push(crossing_at_v(circle[last], circle[after_last], v_min));
    boundary
}

/// The cross-section an arch is built from: the axes it is laid out on, the circle its
/// band follows and the boundary of its outside, running from one foot to the other.
struct ArchCrossSection {
    axes: ArchAxes,
    circle_bounds: Bbox2d,
    circle: Vec<Vec2d>,
    upper: Vec<usize>,
    outer_boundary: Vec<Vec2d>,
}

fn make_arch_cross_section(
    bounds: &Bbox3d,
    circle_shape: CircleShape,
    axis: Axis,
) -> Option<ArchCrossSection> {
    let axes = arch_axes(axis);

    let s_min = bounds.min[axes.span];
    let s_max = bounds.max[axes.span];
    let v_min = bounds.min[axes.vertical];
    let v_max = bounds.max[axes.vertical];
    let height = v_max - v_min;
    let span = s_max - s_min;

    // The bounds are often degenerate mid-drag; emit no brushes rather than erroring.
    if height <= 0.0 || span <= 0.0 {
        return None;
    }

    // Upper half of an ellipse whose diameter lies on the springing line (v == v_min).
    // Build the full ellipse in bounds doubled downwards, then keep only the upper half,
    // reusing the cylinder circle-mode machinery.
    let circle_bounds = Bbox2d::new(Vec2d::new(s_min, v_min - height), Vec2d::new(s_max, v_max));

    let circle = make_circle(circle_shape, &circle_bounds);
    let upper = arch_upper_run(&circle, v_min);
    if upper.is_empty() {
        return None;
    }

    let outer_boundary = arch_boundary(&circle, &upper, v_min);
    Some(ArchCrossSection {
        axes,
        circle_bounds,
        circle,
        upper,
        outer_boundary,
    })
}

fn arch_point(axes: &ArchAxes, p: Vec2d, w: f64) -> Vec3d {
    let mut result = Vec3d::zero();
    result[axes.span] = p.x();
    result[axes.vertical] = p.y();
    result[axes.tunnel] = w;
    result
}

/* ========== spheres ========== */

fn sub_divide_ratios(ratios: &[f64]) -> Vec<f64> {
    let mut new_ratios = Vec::with_capacity(ratios.len() + 1);
    new_ratios.push(ratios[0]);
    for j in 1..ratios.len() {
        new_ratios.push((ratios[j - 1] + ratios[j]) / 2.0);
    }
    new_ratios.push(ratios[ratios.len() - 1]);
    new_ratios
}

/// Mirrors a half-profile into a full one, so a sphere's top half also describes its
/// bottom.
fn mirror_ratios(mut ratios: Vec<f64>, negate: bool) -> Vec<f64> {
    let n = ratios.len();
    for i in 0..n - 1 {
        let value = ratios[n - i - 2];
        ratios.push(if negate { -value } else { value });
    }
    ratios
}

fn make_size_ratios_per_ring(precision: usize) -> Vec<f64> {
    let mut size_ratios = vec![0.0, 1.0 / 2.0, 7.0 / 8.0, 1.0];
    for _ in 0..precision {
        size_ratios = sub_divide_ratios(&size_ratios);
    }
    mirror_ratios(size_ratios, false)
}

fn make_z_ratios_per_ring(precision: usize) -> Vec<f64> {
    let mut z_ratios = vec![1.0, 7.0 / 8.0, 1.0 / 2.0, 0.0];
    for _ in 0..precision {
        z_ratios = sub_divide_ratios(&z_ratios);
    }
    mirror_ratios(z_ratios, true)
}

fn make_scalable_uv_sphere(bounds_xy: &Bbox3d, precision: usize) -> Vec<Vec3d> {
    let z_ratios = make_z_ratios_per_ring(precision);
    let size_ratios = make_size_ratios_per_ring(precision);

    let get_z = |i: usize| {
        let center = bounds_xy.center();
        let size = bounds_xy.size() / 2.0;
        center.z() + size.z() * z_ratios[i]
    };
    let get_bounds = |i: usize| {
        let s = bounds_xy.size().x().min(bounds_xy.size().y()) / 2.0;
        footprint(bounds_xy).expand(-s * (1.0 - size_ratios[i]))
    };

    // Ring count follows the available ratio table, excluding the two tip entries.
    let num_rings = z_ratios.len() - 2;

    let mut vertices = set_z(&make_scalable_cone_tip(bounds_xy), get_z(0));
    for i in 1..=num_rings {
        vertices.extend(set_z(
            &make_scalable_circle(precision, &get_bounds(i)),
            get_z(i),
        ));
    }
    vertices.extend(set_z(
        &make_scalable_cone_tip(bounds_xy),
        get_z(num_rings + 1),
    ));
    vertices
}

/// One ring of a UV sphere, at the given angle down from the pole.
fn make_ring(angle: f64, circle_shape: CircleShape, bounds_xy: &Bbox3d) -> Vec<Vec3d> {
    let r = angle.sin();
    let z = bounds_xy.center().z() + angle.cos() * bounds_xy.size().z() / 2.0;
    let center = footprint(bounds_xy).center();
    let t = translation_matrix_2d(center)
        * scaling_matrix_2d(Vec2d::new(r, r))
        * translation_matrix_2d(-center);

    make_circle(circle_shape, &footprint(bounds_xy))
        .into_iter()
        .map(|v| {
            let v = t * v;
            Vec3d::new(v.x(), v.y(), z)
        })
        .collect()
}

fn make_aligned_uv_sphere(
    bounds_xy: &Bbox3d,
    circle_shape: CircleShape,
    num_rings: usize,
) -> Vec<Vec3d> {
    let angle_delta = std::f64::consts::PI / (num_rings as f64 + 1.0);
    let center = footprint(bounds_xy).center();

    let mut vertices = vec![Vec3d::new(center.x(), center.y(), bounds_xy.max.z())];
    for i in 0..num_rings {
        vertices.extend(make_ring(i as f64 * angle_delta, circle_shape, bounds_xy));
    }
    vertices.push(Vec3d::new(center.x(), center.y(), bounds_xy.min.z()));

    // Ensure that the sphere fills the bounds when the number of rings is even.
    let center_ring_radius = (angle_delta * (num_rings / 2) as f64).sin();
    let extra_scale = if num_rings.is_multiple_of(2) {
        1.0 / center_ring_radius
    } else {
        1.0
    };
    let transform = translation_matrix(bounds_xy.center())
        * scaling_matrix(Vec3d::new(extra_scale, extra_scale, 1.0))
        * translation_matrix(-bounds_xy.center());

    vertices.into_iter().map(|v| transform * v).collect()
}

/// The vertices and triangles of a unit sphere, from a subdivided icosahedron.
///
/// `map_model::sphereMesh`.
fn sphere_mesh(iterations: usize) -> (Vec<Vec3d>, Vec<[usize; 3]>) {
    let t = (1.0 + 5.0f64.sqrt()) / 2.0;
    let mut vertices: Vec<Vec3d> = [
        Vec3d::new(-1.0, t, 0.0),
        Vec3d::new(1.0, t, 0.0),
        Vec3d::new(-1.0, -t, 0.0),
        Vec3d::new(1.0, -t, 0.0),
        Vec3d::new(0.0, -1.0, t),
        Vec3d::new(0.0, 1.0, t),
        Vec3d::new(0.0, -1.0, -t),
        Vec3d::new(0.0, 1.0, -t),
        Vec3d::new(t, 0.0, -1.0),
        Vec3d::new(t, 0.0, 1.0),
        Vec3d::new(-t, 0.0, -1.0),
        Vec3d::new(-t, 0.0, 1.0),
    ]
    .into_iter()
    .map(vec::normalize)
    .collect();

    let mut triangles: Vec<[usize; 3]> = vec![
        // 5 triangles around point 0
        [0, 5, 11],
        [0, 1, 5],
        [0, 7, 1],
        [0, 10, 7],
        [0, 11, 10],
        // 5 adjacent faces
        [4, 11, 5],
        [9, 5, 1],
        [8, 1, 7],
        [6, 7, 10],
        [2, 10, 11],
        // 5 faces around point 3
        [3, 2, 4],
        [3, 6, 2],
        [3, 8, 6],
        [3, 9, 8],
        [3, 4, 9],
        // 5 adjacent faces
        [11, 4, 2],
        [10, 2, 6],
        [7, 6, 8],
        [1, 8, 9],
        [5, 9, 4],
    ];

    let mut cache: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();
    for _ in 0..iterations {
        let mut new_triangles = Vec::with_capacity(triangles.len() * 4);
        for triangle in &triangles {
            let index1 = mid_point(&mut vertices, &mut cache, triangle[0], triangle[1]);
            let index2 = mid_point(&mut vertices, &mut cache, triangle[1], triangle[2]);
            let index3 = mid_point(&mut vertices, &mut cache, triangle[2], triangle[0]);
            new_triangles.push([triangle[0], index1, index3]);
            new_triangles.push([triangle[1], index2, index1]);
            new_triangles.push([triangle[2], index3, index2]);
            new_triangles.push([index1, index2, index3]);
        }
        triangles = new_triangles;
    }

    (vertices, triangles)
}

fn mid_point(
    vertices: &mut Vec<Vec3d>,
    cache: &mut std::collections::HashMap<(usize, usize), usize>,
    index1: usize,
    index2: usize,
) -> usize {
    if let Some(found) = cache.get(&(index1, index2)) {
        return *found;
    }

    let mid = (vertices[index1] + vertices[index2]) / 2.0;
    vertices.push(vec::normalize(mid));

    let mid_index = vertices.len() - 1;
    cache.insert((index1, index2), mid_index);
    cache.insert((index2, index1), mid_index);
    mid_index
}

/* ========== the shapes themselves ========== */

/// Builds a box filling the given bounds.
pub fn create_cuboid(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    material: &str,
) -> Result<Brush, ShapeError> {
    let (min, max) = (bounds.min, bounds.max);
    let faces = [
        // left
        [
            min,
            min + Vec3d::new(0.0, 1.0, 0.0),
            min + Vec3d::new(0.0, 0.0, 1.0),
        ],
        // right
        [
            max,
            max + Vec3d::new(0.0, 0.0, 1.0),
            max + Vec3d::new(0.0, 1.0, 0.0),
        ],
        // front
        [
            min,
            min + Vec3d::new(0.0, 0.0, 1.0),
            min + Vec3d::new(1.0, 0.0, 0.0),
        ],
        // back
        [
            max,
            max + Vec3d::new(1.0, 0.0, 0.0),
            max + Vec3d::new(0.0, 0.0, 1.0),
        ],
        // top
        [
            max,
            max + Vec3d::new(0.0, 1.0, 0.0),
            max + Vec3d::new(1.0, 0.0, 0.0),
        ],
        // bottom
        [
            min,
            min + Vec3d::new(1.0, 0.0, 0.0),
            min + Vec3d::new(0.0, 1.0, 0.0),
        ],
    ];

    let faces = faces
        .into_iter()
        .map(|points| {
            BrushFace::from_points(
                points,
                material,
                UvCoordSystem::Paraxial,
                UvAttributes::default(),
            )
            .map_err(|_| ShapeError::FlatBounds)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Brush::from_faces(world_bounds, faces).map_err(ShapeError::Invalid)
}

/// Builds a wedge -- a ramp filling the given bounds, climbing along `axis` to its
/// full height at that axis's far side.
///
/// Creates a ramp climbing along a horizontal `axis`. The vertical Z axis is invalid.
pub fn create_wedge(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    axis: Axis,
    material: &str,
) -> Result<Brush, ShapeError> {
    if axis == Axis::Z {
        return Err(ShapeError::FlatBounds);
    }
    let (min, max) = (bounds.min, bounds.max);
    // The floor's four corners, and the top edge over the far side of the climb.
    let mut points = vec![
        Vec3d::new(min.x(), min.y(), min.z()),
        Vec3d::new(max.x(), min.y(), min.z()),
        Vec3d::new(min.x(), max.y(), min.z()),
        Vec3d::new(max.x(), max.y(), min.z()),
    ];
    match axis {
        Axis::X => {
            points.push(Vec3d::new(max.x(), min.y(), max.z()));
            points.push(Vec3d::new(max.x(), max.y(), max.z()));
        }
        _ => {
            points.push(Vec3d::new(min.x(), max.y(), max.z()));
            points.push(Vec3d::new(max.x(), max.y(), max.z()));
        }
    }
    create_brush(world_bounds, &points, material)
}

/// Builds a cylinder inscribed in the given bounds, running along the given axis.
pub fn create_cylinder(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    circle_shape: CircleShape,
    axis: Axis,
    material: &str,
) -> Result<Brush, ShapeError> {
    circle_shape.check()?;
    let (to_xy, from_xy) = axis_transforms(axis);

    let cylinder = make_cylinder(circle_shape, &bounds.transform(&to_xy));
    create_brush(
        world_bounds,
        &transform_points(&from_xy, &cylinder),
        material,
    )
}

/// Builds a cylindrical shell, as one brush per segment of its wall.
pub fn create_hollow_cylinder(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    thickness: f64,
    circle_shape: CircleShape,
    axis: Axis,
    material: &str,
) -> Result<Vec<Brush>, ShapeError> {
    circle_shape.check()?;
    let (to_xy, from_xy) = axis_transforms(axis);
    let bounds_xy = bounds.transform(&to_xy);

    let outer_circle = make_circle(circle_shape, &footprint(&bounds_xy));
    let inner_circle = make_hollow_cylinder_inner_circle(
        &outer_circle,
        thickness,
        circle_shape,
        &footprint(&bounds_xy),
    )?;
    debug_assert_eq!(inner_circle.len(), outer_circle.len());

    (0..outer_circle.len())
        .map(|i| {
            let fragment =
                make_hollow_cylinder_fragment_vertices(&outer_circle, &inner_circle, i, &bounds_xy);
            create_brush(
                world_bounds,
                &transform_points(&from_xy, &fragment),
                material,
            )
        })
        .collect()
}

/// Builds an arch, as one brush per voussoir.
pub fn create_arch(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    thickness: f64,
    circle_shape: CircleShape,
    axis: Axis,
    material: &str,
) -> Result<Vec<Brush>, ShapeError> {
    circle_shape.check()?;
    let Some(section) = make_arch_cross_section(bounds, circle_shape, axis) else {
        return Ok(Vec::new());
    };

    let axes = section.axes;
    let v_min = bounds.min[axes.vertical];
    let w_min = bounds.min[axes.tunnel];
    let w_max = bounds.max[axes.tunnel];

    let inner = make_hollow_cylinder_inner_circle(
        &section.circle,
        thickness,
        circle_shape,
        &section.circle_bounds,
    )?;
    debug_assert_eq!(inner.len(), section.circle.len());

    let outer_boundary = &section.outer_boundary;
    let inner_boundary = arch_boundary(&inner, &section.upper, v_min);

    // Build each voussoir independently, skipping any that are degenerate mid-drag.
    Ok((0..outer_boundary.len() - 1)
        .filter_map(|j| {
            let o0 = outer_boundary[j];
            let o1 = outer_boundary[j + 1];
            let i0 = inner_boundary[j];
            let i1 = inner_boundary[j + 1];

            let points = [
                arch_point(&axes, o0, w_min),
                arch_point(&axes, o0, w_max),
                arch_point(&axes, o1, w_min),
                arch_point(&axes, o1, w_max),
                arch_point(&axes, i0, w_min),
                arch_point(&axes, i0, w_max),
                arch_point(&axes, i1, w_min),
                arch_point(&axes, i1, w_max),
            ];
            create_brush(world_bounds, &points, material).ok()
        })
        .collect())
}

/// Builds the solid filling the corners an arch leaves open, as one brush per segment.
pub fn create_spandrel_for_arch(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    circle_shape: CircleShape,
    axis: Axis,
    material: &str,
) -> Result<Vec<Brush>, ShapeError> {
    circle_shape.check()?;
    let Some(section) = make_arch_cross_section(bounds, circle_shape, axis) else {
        return Ok(Vec::new());
    };

    let axes = section.axes;
    let s_min = bounds.min[axes.span];
    let s_max = bounds.max[axes.span];
    let v_min = bounds.min[axes.vertical];
    let v_max = bounds.max[axes.vertical];
    let w_min = bounds.min[axes.tunnel];
    let w_max = bounds.max[axes.tunnel];

    let outer_boundary = &section.outer_boundary;
    let center = Vec2d::new((s_min + s_max) / 2.0, v_min);

    // The outer boundary is convex and touches the top of the bounds, so fanning its
    // segments to the corner they face tiles the gap exactly. Segments that lie on the
    // bounds themselves fan into a degenerate brush and drop out.
    Ok((0..outer_boundary.len() - 1)
        .filter_map(|j| {
            let o0 = outer_boundary[j];
            let o1 = outer_boundary[j + 1];
            let corner = Vec2d::new(
                if outward_normal(o0, o1, center).x() < 0.0 {
                    s_min
                } else {
                    s_max
                },
                v_max,
            );

            let points = [
                arch_point(&axes, o0, w_min),
                arch_point(&axes, o0, w_max),
                arch_point(&axes, o1, w_min),
                arch_point(&axes, o1, w_max),
                arch_point(&axes, corner, w_min),
                arch_point(&axes, corner, w_max),
            ];
            create_brush(world_bounds, &points, material).ok()
        })
        .collect())
}

/// Builds a cone inscribed in the given bounds, pointing along the given axis.
pub fn create_cone(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    circle_shape: CircleShape,
    axis: Axis,
    material: &str,
) -> Result<Brush, ShapeError> {
    circle_shape.check()?;
    let (to_xy, from_xy) = axis_transforms(axis);
    let bounds_xy = bounds.transform(&to_xy);

    let cone = make_cone(circle_shape, &bounds_xy);
    create_brush(world_bounds, &transform_points(&from_xy, &cone), material)
}

/// Builds a sphere from stacked rings, like a globe's parallels.
pub fn create_uv_sphere(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    circle_shape: CircleShape,
    num_rings: usize,
    axis: Axis,
    material: &str,
) -> Result<Brush, ShapeError> {
    circle_shape.check()?;
    if num_rings == 0 {
        return Err(ShapeError::TooFewSides { given: num_rings });
    }
    let (to_xy, from_xy) = axis_transforms(axis);
    let bounds_xy = bounds.transform(&to_xy);

    let sphere = match circle_shape {
        CircleShape::Scalable { precision } => make_scalable_uv_sphere(&bounds_xy, precision),
        _ => make_aligned_uv_sphere(&bounds_xy, circle_shape, num_rings),
    };

    create_brush(world_bounds, &transform_points(&from_xy, &sphere), material)
}

/// Builds a sphere by subdividing an icosahedron, so every face is about the same size.
pub fn create_ico_sphere(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    iterations: usize,
    material: &str,
) -> Result<Brush, ShapeError> {
    let (sphere_vertices, _) = sphere_mesh(iterations);

    // The mesh is a unit sphere about the origin; this maps [-1, 1] onto the bounds.
    //
    // Constructs the solid by hulling the sphere vertices, avoiding accumulated precision loss from
    // clipping a large world box against many closely spaced planes.
    let transform = translation_matrix(bounds.min)
        * scaling_matrix(bounds.size())
        * scaling_matrix(Vec3d::new(0.5, 0.5, 0.5))
        * translation_matrix(Vec3d::new(1.0, 1.0, 1.0));

    create_brush(
        world_bounds,
        &transform_points(&transform, &sphere_vertices),
        material,
    )
}

/// A centrally symmetric sphere mesh, refined by bisecting a longest edge and
/// its antipodal partner. Both triangles sharing each edge are split together,
/// so every step adds four triangles without T junctions or open contacts.
fn shell_mesh(brush_count: usize) -> Result<(Vec<Vec3d>, Vec<[usize; 3]>), ShapeError> {
    let (mut vertices, mut triangles) = sphere_mesh(0);
    let mut opposite: Vec<_> = vertices
        .iter()
        .map(|v| {
            vertices
                .iter()
                .position(|w| vec::length(*v + *w) < 1e-12)
                .expect("antipodal icosahedron")
        })
        .collect();
    while triangles.len() < brush_count {
        let sorted = |a: usize, b: usize| if a < b { (a, b) } else { (b, a) };
        let length =
            |edge: (usize, usize)| vec::squared_length(vertices[edge.0] - vertices[edge.1]);
        let edge = triangles
            .iter()
            .flat_map(|t| [sorted(t[0], t[1]), sorted(t[1], t[2]), sorted(t[2], t[0])])
            .min_by(|a, b| length(*b).total_cmp(&length(*a)).then_with(|| a.cmp(b)))
            .expect("closed sphere edges");
        let anti = sorted(opposite[edge.0], opposite[edge.1]);
        let midpoint = vec::normalize(vertices[edge.0] + vertices[edge.1]);
        let mid = vertices.len();
        vertices.extend([midpoint, -midpoint]);
        opposite.extend([mid + 1, mid]);
        let mut next = Vec::with_capacity(triangles.len() + 4);
        for triangle in triangles {
            if let Some((i, m)) = (0..3).find_map(|i| {
                let candidate = sorted(triangle[i], triangle[(i + 1) % 3]);
                if candidate == edge {
                    Some((i, mid))
                } else if candidate == anti {
                    Some((i, mid + 1))
                } else {
                    None
                }
            }) {
                let [a, b, c] = [triangle[i], triangle[(i + 1) % 3], triangle[(i + 2) % 3]];
                next.extend([[a, m, c], [m, b, c]]);
            } else {
                next.push(triangle);
            }
        }
        debug_assert_eq!(next.len() % 4, 0);
        triangles = next;
    }
    // Hull the unit vertices once after refinement. Merely splitting curved edges
    // can leave a concave diagonal between four samples of the sphere.
    let hull = Polyhedron::from_points(&vertices, f64::ALMOST_ZERO).map_err(ShapeError::Hull)?;
    let mut surface = Vec::with_capacity(brush_count);
    for face in 0..hull.face_count() {
        let polygon = hull.face_positions(crate::polyhedron::FaceId(face as u32));
        let mut indices = polygon
            .iter()
            .map(|p| {
                vertices
                    .iter()
                    .position(|v| vec::length(*v - *p) < 1e-12)
                    .ok_or(ShapeError::InvalidParameter(
                        "sphere hull changed its sample vertices",
                    ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Coplanar hull polygons must use antipodally matching fan diagonals too.
        let first = (0..indices.len())
            .min_by_key(|&i| indices[i].min(opposite[indices[i]]))
            .unwrap();
        indices.rotate_left(first);
        for i in 1..indices.len() - 1 {
            surface.push([indices[0], indices[i], indices[i + 1]]);
        }
    }
    if surface.len() != brush_count {
        return Err(ShapeError::InvalidParameter(
            "sphere hull lost a refinement vertex",
        ));
    }
    Ok((vertices, surface))
}

/// A closed geodesic shell. Each triangle becomes a convex six-vertex frustum,
/// sharing its three radial side faces exactly with neighbouring brushes.
/// Thickness is radial at the shortest semi-axis; stretched bounds make an ellipsoid.
/// The explicit brush count is a multiple of four from 20 to 320.
pub fn create_hollow_sphere(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    thickness: f64,
    brush_count: usize,
    material: &str,
) -> Result<Vec<Brush>, ShapeError> {
    if !(20..=320).contains(&brush_count) || !brush_count.is_multiple_of(4) {
        return Err(ShapeError::InvalidParameter(
            "hollow sphere brush count must be a multiple of four from 20 to 320",
        ));
    }
    let size = bounds.size();
    if (0..3).any(|i| {
        !bounds.min[i].is_finite()
            || !bounds.max[i].is_finite()
            || !size[i].is_finite()
            || size[i] <= 0.0
    }) {
        return Err(ShapeError::FlatBounds);
    }
    let radius = size[0].min(size[1]).min(size[2]) * 0.5;
    if !thickness.is_finite() || thickness <= 0.0 || thickness >= radius {
        return Err(ShapeError::InvalidParameter(
            "wall thickness must be positive and smaller than the shortest radius",
        ));
    }
    let center = bounds.min + size * 0.5;
    let ratio = 1.0 - thickness / radius;
    let (vertices, triangles) = shell_mesh(brush_count)?;
    let outer: Vec<_> = vertices
        .iter()
        .map(|v| {
            center
                + Vec3d::new(
                    v[0] * size[0] * 0.5,
                    v[1] * size[1] * 0.5,
                    v[2] * size[2] * 0.5,
                )
        })
        .collect();
    if outer.iter().any(|p| !world_bounds.contains_point(*p)) {
        return Err(ShapeError::InvalidParameter(
            "the hollow sphere must fit inside the world bounds",
        ));
    }
    let inner: Vec<_> = outer
        .iter()
        .map(|p| center + (*p - center) * ratio)
        .collect();
    let mut result = Vec::with_capacity(triangles.len());
    for indices in triangles {
        let points: Vec<_> = indices.iter().flat_map(|&i| [outer[i], inner[i]]).collect();
        let brush = create_brush(world_bounds, &points, material)?;
        if brush.faces().len() != 5 || brush.volume() <= 1e-9 {
            return Err(ShapeError::InvalidParameter(
                "the shell cells are too small; increase the size or reduce the brush count",
            ));
        }
        let mut faces = brush.faces().to_vec();
        for face in &mut faces {
            // A radial contact plane passes through the common shell centre.
            if (vec::dot(face.normal(), center) - face.plane.distance).abs() < 1e-7 {
                face.material_name = "skip".into();
            }
        }
        if faces
            .iter()
            .filter(|f| f.material_name.as_str() == "skip")
            .count()
            != 3
            && material != "skip"
        {
            return Err(ShapeError::InvalidParameter(
                "the shell is too small to distinguish its inner and outer surfaces",
            ));
        }
        result.push(brush.with_face_attributes_from(&faces));
    }
    Ok(result)
}

/// Builds a flight of steps filling the bounds, treads running along `axis`.
///
/// `DrawShapeToolStairsExtension::createBrushes`. Each step is a box, so this is the one
/// shape whose result is several brushes for a reason other than convexity.
pub fn create_stairs(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    step_height: f64,
    axis: Axis,
    ascending: bool,
    material: &str,
) -> Result<Vec<Brush>, ShapeError> {
    let step_height = step_height.abs().max(1.0);
    let axis = axis.index();
    let size = bounds.size();
    if size.z() <= 0.0 || size[axis] <= 0.0 {
        return Ok(Vec::new());
    }

    let num_steps = ((size.z() / step_height).ceil() as usize).max(1);
    let tread_depth = size[axis] / num_steps as f64;

    (0..num_steps)
        .map(|i| {
            let mut step_bounds = Bbox3d::new(
                bounds.min,
                Vec3d::new(
                    bounds.max.x(),
                    bounds.max.y(),
                    (bounds.min.z() + step_height * (i + 1) as f64).min(bounds.max.z()),
                ),
            );

            let (tread_min, tread_max) = if ascending {
                (
                    bounds.min[axis] + tread_depth * i as f64,
                    bounds.min[axis] + tread_depth * (i + 1) as f64,
                )
            } else {
                (
                    bounds.max[axis] - tread_depth * (i + 1) as f64,
                    bounds.max[axis] - tread_depth * i as f64,
                )
            };

            step_bounds.min[axis] = tread_min;
            step_bounds.max[axis] = tread_max;

            create_cuboid(world_bounds, &step_bounds, material)
        })
        .collect()
}

/* ========== shared plumbing ========== */

/// The transforms taking the given axis to Z and back.
fn axis_transforms(axis: Axis) -> (Mat4x4d, Mat4x4d) {
    let mut along = Vec3d::zero();
    along[axis.index()] = 1.0;
    let z = Vec3d::new(0.0, 0.0, 1.0);
    (
        rotation_matrix_from_to(along, z),
        rotation_matrix_from_to(z, along),
    )
}

fn transform_points(transform: &Mat4x4d, points: &[Vec3d]) -> Vec<Vec3d> {
    points.iter().map(|p| *transform * *p).collect()
}

/// Hulls the given points and makes a brush of the result.
///
/// `BrushBuilder::createBrush`.
fn create_brush(
    _world_bounds: &Bbox3d,
    points: &[Vec3d],
    material: &str,
) -> Result<Brush, ShapeError> {
    let geometry = Polyhedron::from_points(points, f64::ALMOST_ZERO).map_err(ShapeError::Hull)?;
    Ok(Brush::from_polyhedron(geometry, material))
}

#[path = "advanced_stairs.rs"]
mod advanced_stairs;
pub use advanced_stairs::{create_stairs_options, StairLayout, StairOptions};

#[cfg(test)]
mod shell_mesh_tests {
    use super::*;
    #[test]
    fn every_shell_count_has_closed_convex_antipodal_topology() {
        use std::collections::{BTreeMap, BTreeSet};
        for count in (20..=320).step_by(4) {
            let (vertices, triangles) = shell_mesh(count).unwrap();
            assert_eq!(triangles.len(), count);
            assert_eq!(vertices.len(), count / 2 + 2);
            let opposite: Vec<_> = vertices
                .iter()
                .map(|v| {
                    vertices
                        .iter()
                        .position(|w| vec::length(*v + *w) < 1e-12)
                        .unwrap()
                })
                .collect();
            let faces: BTreeSet<_> = triangles
                .iter()
                .map(|t| {
                    let mut t = *t;
                    t.sort();
                    t
                })
                .collect();
            assert_eq!(faces.len(), count);
            let mut edges = BTreeMap::new();
            for triangle in &triangles {
                let mut anti = triangle.map(|i| opposite[i]);
                anti.sort();
                assert!(faces.contains(&anti), "{count}: asymmetric triangle");
                let [a, b, c] = triangle.map(|i| vertices[i]);
                let mut normal = vec::normalize(vec::cross(b - a, c - a));
                if vec::dot(normal, a) < 0.0 {
                    normal = -normal;
                }
                assert!(
                    vertices.iter().all(|p| vec::dot(normal, *p - a) < 1e-9),
                    "{count}: concave outer surface"
                );
                for i in 0..3 {
                    let a = triangle[i];
                    let b = triangle[(i + 1) % 3];
                    *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
                }
            }
            assert!(
                edges.values().all(|n| *n == 2),
                "{count}: open edge or T junction"
            );
        }
    }
    #[test]
    fn shell_refinement_is_repeatable() {
        for count in [24, 84, 320] {
            assert_eq!(shell_mesh(count).unwrap(), shell_mesh(count).unwrap());
        }
    }
}
