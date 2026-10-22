//! How a material is mapped onto a brush face.
//!
//! There are two systems, and which one a face uses is decided by the map format:
//!
//! - **Paraxial**, used by the Standard format. The UV axes are not stored in the file;
//!   they are looked up from the coordinate plane the face is most nearly parallel to,
//!   then rotated. A face at an angle to all three axes therefore shows a stretched
//!   material — the well known Quake behaviour.
//! - **Parallel**, used by the Valve 220 format. The UV axes are stored in the file and
//!   lie in the face's own plane, so the mapping is undistorted at any angle.

use geometry_math::scalar;
use geometry_math::scalar::Float;
use geometry_math::vec::{self, Vec2f, Vec3d};

/// The offset, scale and rotation applied on top of a face's UV axes.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UvAttributes {
    /// The material offset, in texels.
    pub offset: Vec2f,
    /// The material scale. A scale of 1 maps one texel to one world unit.
    pub scale: Vec2f,
    /// The material rotation, in degrees.
    pub rotation: f32,
}

impl Default for UvAttributes {
    fn default() -> Self {
        Self {
            offset: Vec2f::zero(),
            scale: Vec2f::new(1.0, 1.0),
            rotation: 0.0,
        }
    }
}

/// Returns a scale factor safe to divide by.
///
/// Treats zero UV scale as one to avoid infinities when reading malformed map data.
#[inline]
pub fn safe_scale(value: f32) -> f32 {
    if value == 0.0 {
        1.0
    } else {
        value
    }
}

/// The six base axis triples the paraxial system chooses from.
///
/// Each triple is a plane normal followed by its U and V axes. The order matters: it is
/// the order Quake's own tools use, and changing it would change how existing maps look.
const BASE_AXES: [[Vec3d; 3]; 6] = [
    // Floor
    [
        Vec3d::new(0.0, 0.0, 1.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, -1.0, 0.0),
    ],
    // Ceiling
    [
        Vec3d::new(0.0, 0.0, -1.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, -1.0, 0.0),
    ],
    // West wall
    [
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
        Vec3d::new(0.0, 0.0, -1.0),
    ],
    // East wall
    [
        Vec3d::new(-1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
        Vec3d::new(0.0, 0.0, -1.0),
    ],
    // South wall
    [
        Vec3d::new(0.0, 1.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, 0.0, -1.0),
    ],
    // North wall
    [
        Vec3d::new(0.0, -1.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, 0.0, -1.0),
    ],
];

/// Returns which of the six base axis triples best matches the given face normal.
pub fn plane_normal_index(normal: Vec3d) -> usize {
    let mut best_index = 0;
    let mut best_dot = 0.0;
    for (index, axes) in BASE_AXES.iter().enumerate() {
        let dot = vec::dot(normal, axes[0]);
        if dot > best_dot {
            best_dot = dot;
            best_index = index;
        }
    }
    best_index
}

/// Returns the unrotated U and V axes for the given base axis index.
#[inline]
pub fn base_axes(index: usize) -> (Vec3d, Vec3d) {
    (BASE_AXES[index][1], BASE_AXES[index][2])
}

/// How a face's UV axes are determined.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UvCoordSystem {
    /// The axes come from the face's dominant coordinate plane, rotated by the
    /// attributes' rotation. This is the Standard format's behaviour.
    Paraxial,
    /// The axes are given explicitly and lie in the face's own plane. This is what the
    /// Valve 220 format stores.
    Parallel {
        /// The U axis.
        u_axis: Vec3d,
        /// The V axis.
        v_axis: Vec3d,
    },
}

impl UvCoordSystem {
    /// Returns the U and V axes this system maps with, for a face with the given normal
    /// and attributes.
    ///
    /// The axes are unscaled; [`uv_coords`](UvCoordSystem::uv_coords) applies the scale.
    pub fn axes(&self, normal: Vec3d, attributes: &UvAttributes) -> (Vec3d, Vec3d) {
        match self {
            UvCoordSystem::Paraxial => {
                let index = plane_normal_index(normal);
                let (u_axis, v_axis) = base_axes(index);
                rotate_paraxial_axes(u_axis, v_axis, attributes.rotation, index)
            }
            // The rotation is already baked into the stored axes.
            UvCoordSystem::Parallel { u_axis, v_axis } => (*u_axis, *v_axis),
        }
    }

    /// Returns this system rotated by the given angle about the face's normal.
    ///
    /// The paraxial system keeps its rotation as an attribute, so nothing changes here;
    /// the parallel system stores its axes, so they turn.
    pub fn rotated(&self, normal: Vec3d, degrees: f32) -> Self {
        match self {
            UvCoordSystem::Paraxial => UvCoordSystem::Paraxial,
            UvCoordSystem::Parallel { u_axis, v_axis } => {
                let quat = geometry_math::quat::Quatd::from_axis_angle(
                    normal,
                    scalar::to_radians(f64::from(degrees)),
                );
                UvCoordSystem::Parallel {
                    u_axis: vec::correct(quat * *u_axis, 0, AXIS_CORRECT_EPSILON),
                    v_axis: vec::correct(quat * *v_axis, 0, AXIS_CORRECT_EPSILON),
                }
            }
        }
    }

    /// Returns this system with the material skewed, `ParallelUvCoordSystem::shear`.
    ///
    /// The first factor leans V towards U and the second leans U towards V. Paraxial mappings
    /// cannot represent shear and remain unchanged.
    pub fn sheared(&self, normal: Vec3d, factors: Vec2f) -> Self {
        self.skewed(
            normal,
            [[1.0, f64::from(factors.x())], [f64::from(factors.y()), 1.0]],
        )
    }

    /// Returns this system with the given 2x2 matrix applied to its axes in the face's
    /// own UV plane: the general step `sheared` is one case of.
    ///
    /// A drag that skews the material a little further each step cannot add its shears
    /// up -- two shears compose to something that is not a shear -- so the UV editor
    /// applies the inverse of what it applied last and then the whole of what it wants
    /// now, and that product is what this takes. `rows` is row-major: the new U
    /// coordinate is `rows[0]` dotted with the old (U, V).
    pub fn skewed(&self, normal: Vec3d, rows: [[f64; 2]; 2]) -> Self {
        let UvCoordSystem::Parallel { u_axis, v_axis } = self else {
            return *self;
        };
        let Some(to_uv) = geometry_math::mat_ext::coordinate_system_matrix(
            *u_axis,
            *v_axis,
            normal,
            Vec3d::zero(),
        ) else {
            return *self;
        };
        let Some(from_uv) = geometry_math::mat::invert(to_uv) else {
            return *self;
        };
        let skew = geometry_math::mat::Matrix::<f64, 4, 4>::from_rows([
            [rows[0][0], rows[0][1], 0.0, 0.0],
            [rows[1][0], rows[1][1], 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let transform = from_uv * skew * to_uv;
        UvCoordSystem::Parallel {
            u_axis: transform * *u_axis,
            v_axis: transform * *v_axis,
        }
    }

    /// Returns this system carried onto a face that faces a different way.
    ///
    /// `ParallelUvCoordSystem::updateNormalWithProjection` and
    /// `updateNormalWithRotation`, which is what the two wrap styles choose between when
    /// a mapping is copied from one face to another.
    ///
    /// Paraxial mappings derive their axes from the dominant world plane and retain their stored
    /// rotation.
    pub fn updated_for_normal(
        &self,
        old_normal: Vec3d,
        new_normal: Vec3d,
        style: WrapStyle,
    ) -> Self {
        let UvCoordSystem::Parallel { u_axis, v_axis } = self else {
            return *self;
        };

        match style {
            WrapStyle::Projection => project_axes(*u_axis, *v_axis, new_normal),
            WrapStyle::Rotation => rotate_axes(*u_axis, *v_axis, old_normal, new_normal),
        }
    }

    /// Returns this system reset to the face's default alignment.
    ///
    /// For the parallel system that means axes lying in the face's own plane, which is
    /// what makes a Valve 220 face look undistorted whatever angle it is at.
    pub fn reset(&self, normal: Vec3d) -> Self {
        match self {
            UvCoordSystem::Paraxial => UvCoordSystem::Paraxial,
            UvCoordSystem::Parallel { .. } => {
                let (u_axis, v_axis) = parallel_axes(normal);
                UvCoordSystem::Parallel { u_axis, v_axis }
            }
        }
    }

    /// Returns the UV coordinates of the given point on a face with the given normal, in
    /// texels.
    ///
    /// Divide by the material's dimensions to get the normalized coordinates a renderer
    /// wants.
    pub fn uv_coords(&self, point: Vec3d, normal: Vec3d, attributes: &UvAttributes) -> Vec2f {
        let (u_axis, v_axis) = self.axes(normal, attributes);
        let u = vec::dot(point, u_axis) / f64::from(safe_scale(attributes.scale.x()));
        let v = vec::dot(point, v_axis) / f64::from(safe_scale(attributes.scale.y()));
        Vec2f::new(u as f32, v as f32) + attributes.offset
    }
}

/// The offset brought back into the material's own repeat.
///
/// `modOffset`: an offset of one whole material's width means the same as none at all, so
/// it is kept small rather than growing without bound as a brush is dragged.
pub fn mod_offset(offset: Vec2f, size: Vec2f) -> Vec2f {
    offset - vec::snap_down(offset, size)
}

/// The two in-plane basis vectors a brush primitive measures its texture matrix in.
///
/// `ComputeAxisBase` from the Quake 3 map tool chain (Q3Radiant and its successors): the world is turned so the
/// face normal points down `-Z`, and the S and T axes are where world `+Y` and `-Z` land.
/// Tiny components are zeroed first so an axis-aligned face gets exact axes.
pub fn primitive_axis_base(normal: Vec3d) -> (Vec3d, Vec3d) {
    let clean = |v: f64| if v.abs() < 1e-6 { 0.0 } else { v };
    let (nx, ny, nz) = (clean(normal.x()), clean(normal.y()), clean(normal.z()));

    let rot_y = -f64::atan2(nz, (nx * nx + ny * ny).sqrt());
    let rot_z = f64::atan2(ny, nx);

    let tex_s = Vec3d::new(-rot_z.sin(), rot_z.cos(), 0.0);
    let tex_t = Vec3d::new(
        -rot_y.sin() * rot_z.cos(),
        -rot_y.sin() * rot_z.sin(),
        -rot_y.cos(),
    );
    (tex_s, tex_t)
}

/// Turns a brush primitive's `[[a, b, c], [d, e, f]]` texture matrix into the parallel UV
/// system the editor uses everywhere else.
///
/// The matrix gives normalised texture coordinates as
/// `s = a·S + b·T + c`, `t = d·S + e·T + f`, where `S`, `T` are a point's coordinates in
/// [`primitive_axis_base`]. Multiplying through by the material's pixel size folds it into
/// UV axes measured in texels, which is what [`UvCoordSystem::Parallel`] holds; the scale
/// and rotation attributes come out neutral because the axes carry everything.
pub fn parallel_from_primitive_matrix(
    matrix: [[f64; 3]; 2],
    normal: Vec3d,
    material_size: Vec2f,
) -> (UvCoordSystem, UvAttributes) {
    let (tex_s, tex_t) = primitive_axis_base(normal);
    let width = f64::from(material_size.x().max(1.0));
    let height = f64::from(material_size.y().max(1.0));

    let [[a, b, c], [d, e, f]] = matrix;
    let u_axis = (tex_s * a + tex_t * b) * width;
    let v_axis = (tex_s * d + tex_t * e) * height;

    (
        UvCoordSystem::Parallel { u_axis, v_axis },
        UvAttributes {
            offset: Vec2f::new((c * width) as f32, (f * height) as f32),
            scale: Vec2f::new(1.0, 1.0),
            rotation: 0.0,
        },
    )
}

/// An axis scaled by a factor, with a zero factor taken as one.
fn safe_scale_axis(axis: Vec3d, scale: f32) -> Vec3d {
    axis / f64::from(safe_scale(scale))
}

/// The matrix taking a world point to its UV coordinates.
///
/// `computeWorldToUvMatrix`. The third row is the face's normal, which carries the
/// distance from the plane along; it is what makes the matrix invertible.
fn world_to_uv_matrix(
    u_axis: Vec3d,
    v_axis: Vec3d,
    normal: Vec3d,
    offset: Vec2f,
    scale: Vec2f,
) -> geometry_math::mat::Mat4x4d {
    let u = safe_scale_axis(u_axis, scale.x());
    let v = safe_scale_axis(v_axis, scale.y());
    geometry_math::mat::Mat4x4d::from_rows([
        [u.x(), u.y(), u.z(), f64::from(offset.x())],
        [v.x(), v.y(), v.z(), f64::from(offset.y())],
        [normal.x(), normal.y(), normal.z(), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ])
}

/// How far the UV axes turn about the face's normal under the given transformation.
///
/// `ParallelUvCoordSystem::computeRotationAngle`.
fn parallel_rotation_angle(
    u_axis: Vec3d,
    v_axis: Vec3d,
    old_normal: Vec3d,
    transformation: &geometry_math::mat::Mat4x4d,
) -> f32 {
    let rotation_scale = geometry_math::mat_ext::strip_translation(transformation);
    let new_normal = vec::normalize(rotation_scale * old_normal);

    let non_uv_rotation = geometry_math::quat::Quatd::from_to(old_normal, new_normal);
    let new_u_axis = vec::normalize(rotation_scale * u_axis);
    let non_u_axis = vec::normalize(non_uv_rotation * v_axis);
    scalar::to_degrees(vec::measure_angle_3d(non_u_axis, new_u_axis, new_normal)) as f32
}

impl UvCoordSystem {
    /// Returns this system, and its attributes, carried through a transformation of the
    /// face.
    ///
    /// `ParaxialUvCoordSystem::transform` and `ParallelUvCoordSystem::transform`. With
    /// `lock_alignment` set the material stays put on the face as the face moves, which
    /// is what `Preferences::AlignmentLock` asks for; without it the material stays put
    /// in the world and the face slides across it.
    ///
    /// `old_invariant` is the point that must keep its UV coordinates -- the face's
    /// centre -- and `material_size` is what the offset is kept inside.
    #[allow(
        clippy::too_many_arguments,
        reason = "UV transforms combine mapping, geometry, texture dimensions and an invariant point"
    )]
    pub fn transformed(
        &self,
        attributes: &UvAttributes,
        old_boundary: &geometry_math::plane::Plane3d,
        new_boundary: &geometry_math::plane::Plane3d,
        transformation: &geometry_math::mat::Mat4x4d,
        material_size: Vec2f,
        lock_alignment: bool,
        old_invariant: Vec3d,
    ) -> (Self, UvAttributes) {
        // A mapping with no scale has no axes to carry, and dividing by it would give
        // infinities.
        if attributes.scale.x() == 0.0 || attributes.scale.y() == 0.0 {
            return (*self, *attributes);
        }

        match self {
            UvCoordSystem::Paraxial => {
                // The paraxial axes come from whichever coordinate plane the face most
                // faces, so there is nothing to carry but the rotation, the scale and the
                // offset.
                if !lock_alignment {
                    return (*self, *attributes);
                }
                self.transformed_paraxial(
                    attributes,
                    old_boundary,
                    new_boundary,
                    transformation,
                    material_size,
                    old_invariant,
                )
            }
            UvCoordSystem::Parallel { u_axis, v_axis } => {
                if !lock_alignment {
                    // Without the lock the axes are simply projected onto the new plane,
                    // which is `updateNormalWithProjection`.
                    return (
                        project_axes(*u_axis, *v_axis, new_boundary.normal),
                        *attributes,
                    );
                }
                self.transformed_parallel(
                    *u_axis,
                    *v_axis,
                    attributes,
                    old_boundary,
                    transformation,
                    material_size,
                    old_invariant,
                )
            }
        }
    }

    /// `ParaxialUvCoordSystem::transform`, the locked half.
    fn transformed_paraxial(
        &self,
        attributes: &UvAttributes,
        old_boundary: &geometry_math::plane::Plane3d,
        new_boundary: &geometry_math::plane::Plane3d,
        transformation: &geometry_math::mat::Mat4x4d,
        material_size: Vec2f,
        old_invariant: Vec3d,
    ) -> (Self, UvAttributes) {
        let offset = *transformation * Vec3d::zero();

        // A normal a rounding error away from the old one is the old one; taking it as
        // new would turn the material by a fraction of a degree every time a brush moved.
        let mut new_normal = new_boundary.normal;
        if vec::is_equal(&new_normal, &old_boundary.normal, 0.01) {
            new_normal = old_boundary.normal;
        }

        let old_normal = old_boundary.normal;
        let (u_axis, v_axis) = self.axes(old_normal, attributes);
        let old_invariant_uv = self.uv_coords(old_invariant, old_normal, attributes);

        // The UV axes projected onto the face's own plane, along the face's normal.
        let scale = attributes.scale;
        let origin_on_plane = project_along(old_boundary, Vec3d::zero(), old_normal);
        let u_on_plane = project_along(old_boundary, u_axis * f64::from(scale.x()), old_normal);
        let v_on_plane = project_along(old_boundary, v_axis * f64::from(scale.y()), old_normal);
        let (Some(origin_on_plane), Some(u_on_plane), Some(v_on_plane)) =
            (origin_on_plane, u_on_plane, v_on_plane)
        else {
            return (*self, *attributes);
        };

        // Transformed, with the translation taken back out: an axis is a direction.
        let transformed_u = *transformation * (u_on_plane - origin_on_plane) - offset;
        let transformed_v = *transformation * (v_on_plane - origin_on_plane) - offset;

        // The wider side of the material is the one worth keeping straight.
        let prefer_u = material_size.x() >= material_size.y();

        let new_index = plane_normal_index(new_normal);
        let (base_u, base_v) = base_axes(new_index);
        let uv_normal = BASE_AXES[new_index][0];

        // Onto the new material plane, which is the one the base axes lie in.
        let projected_u = transformed_u - uv_normal * vec::dot(transformed_u, uv_normal);
        let projected_v = transformed_v - uv_normal * vec::dot(transformed_v, uv_normal);
        let normalized_u = vec::normalize(projected_u);
        let normalized_v = vec::normalize(projected_v);

        let angle_between = |base: Vec3d, axis: Vec3d| {
            let mut radians = vec::dot(base, axis).clamp(-1.0, 1.0).acos();
            if vec::dot(vec::cross(base, axis), uv_normal) < 0.0 {
                radians = -radians;
            }
            radians
        };
        let mut radians = if prefer_u {
            angle_between(base_u, normalized_u)
        } else {
            angle_between(base_v, normalized_v)
        };
        // With the material plane's normal on the Y axis the rotation runs the other way.
        if (new_index / 2) * 6 == 12 {
            radians = -radians;
        }

        let new_rotation = scalar::correct(
            scalar::normalize_degrees(scalar::to_degrees(radians)),
            4,
            0.0,
        ) as f32;

        let mut new_scale = Vec2f::new(
            scalar::correct(vec::length(projected_u), 4, 0.0) as f32,
            scalar::correct(vec::length(projected_v), 4, 0.0) as f32,
        );
        // The sign says which way round the axis ended up.
        if vec::dot(u_axis, normalized_u) < 0.0 {
            new_scale = Vec2f::new(-new_scale.x(), new_scale.y());
        }
        if vec::dot(v_axis, normalized_v) < 0.0 {
            new_scale = Vec2f::new(new_scale.x(), -new_scale.y());
        }
        if new_scale.x() == 0.0 || new_scale.y() == 0.0 {
            return (*self, *attributes);
        }

        // The offset is whatever holds the invariant point still.
        let turned = UvAttributes {
            offset: Vec2f::new(0.0, 0.0),
            scale: new_scale,
            rotation: new_rotation,
        };
        let new_invariant = *transformation * old_invariant;
        let new_invariant_uv = self.uv_coords(new_invariant, new_normal, &turned);
        let new_offset = correct_offset(mod_offset(
            old_invariant_uv - new_invariant_uv,
            material_size,
        ));

        (
            *self,
            UvAttributes {
                offset: new_offset,
                scale: new_scale,
                rotation: new_rotation,
            },
        )
    }

    /// `ParallelUvCoordSystem::transform`, the locked half.
    #[allow(
        clippy::too_many_arguments,
        reason = "UV transforms combine mapping, geometry, texture dimensions and an invariant point"
    )]
    fn transformed_parallel(
        &self,
        u_axis: Vec3d,
        v_axis: Vec3d,
        attributes: &UvAttributes,
        old_boundary: &geometry_math::plane::Plane3d,
        transformation: &geometry_math::mat::Mat4x4d,
        material_size: Vec2f,
        old_invariant: Vec3d,
    ) -> (Self, UvAttributes) {
        let angle_delta =
            parallel_rotation_angle(u_axis, v_axis, old_boundary.normal, transformation);
        let new_rotation = scalar::correct(
            scalar::normalize_degrees(f64::from(attributes.rotation + angle_delta)),
            4,
            0.0,
        ) as f32;

        let old_invariant_uv = self.uv_coords(old_invariant, old_boundary.normal, attributes);

        // The mapping is `uv = world_to_uv * point`. After the transformation the same
        // point has to give the same coordinates, so the new matrix is the old one
        // composed with the inverse of the transformation.
        let world_to_uv = world_to_uv_matrix(
            u_axis,
            v_axis,
            old_boundary.normal,
            Vec2f::new(0.0, 0.0),
            Vec2f::new(1.0, 1.0),
        );
        let Some(inverse) = geometry_math::mat::invert(*transformation) else {
            return (*self, *attributes);
        };
        let new_world_to_uv = world_to_uv * inverse;

        // The first two rows of that matrix are the axes.
        let new_u = Vec3d::new(
            new_world_to_uv[0][0],
            new_world_to_uv[1][0],
            new_world_to_uv[2][0],
        );
        let new_v = Vec3d::new(
            new_world_to_uv[0][1],
            new_world_to_uv[1][1],
            new_world_to_uv[2][1],
        );

        let turned = UvCoordSystem::Parallel {
            u_axis: new_u,
            v_axis: new_v,
        };
        let with_rotation = UvAttributes {
            offset: Vec2f::new(0.0, 0.0),
            scale: attributes.scale,
            rotation: new_rotation,
        };
        let new_invariant = *transformation * old_invariant;
        // The axes are stored, so the normal passed here is not read.
        let new_invariant_uv = turned.uv_coords(new_invariant, old_boundary.normal, &with_rotation);
        let new_offset = correct_offset(mod_offset(
            old_invariant_uv - new_invariant_uv,
            material_size,
        ));

        (
            turned,
            UvAttributes {
                offset: new_offset,
                scale: attributes.scale,
                rotation: new_rotation,
            },
        )
    }
}

/// Rounds an offset to four decimals, as `geometry_math::correct` does before it is stored.
fn correct_offset(offset: Vec2f) -> Vec2f {
    Vec2f::new(
        scalar::correct(f64::from(offset.x()), 4, 0.0) as f32,
        scalar::correct(f64::from(offset.y()), 4, 0.0) as f32,
    )
}

/// Where the given point lands on the plane when pushed along the given direction.
///
/// `plane::project_point` with a direction, which returns nothing when the direction runs
/// along the plane rather than into it.
fn project_along(
    plane: &geometry_math::plane::Plane3d,
    point: Vec3d,
    direction: Vec3d,
) -> Option<Vec3d> {
    let denominator = vec::dot(plane.normal, direction);
    if scalar::is_zero(denominator, f64::ALMOST_ZERO) {
        return None;
    }
    let distance = (plane.distance - vec::dot(plane.normal, point)) / denominator;
    Some(point + direction * distance)
}

/// Rotates the paraxial U and V axes about the base plane's normal.
///
/// The rotation axis is the cross product of the base V and U axes rather than the plane
/// normal itself, which is what makes positive rotation turn the material the way the
/// editor's UV view shows.
fn rotate_paraxial_axes(
    u_axis: Vec3d,
    v_axis: Vec3d,
    rotation_degrees: f32,
    index: usize,
) -> (Vec3d, Vec3d) {
    if rotation_degrees == 0.0 {
        return (u_axis, v_axis);
    }

    let rotation_axis = vec::cross(BASE_AXES[index][2], BASE_AXES[index][1]);
    let quat = geometry_math::quat::Quatd::from_axis_angle(
        vec::normalize(rotation_axis),
        scalar::to_radians(f64::from(rotation_degrees)),
    );
    // Correcting snaps values that are a rounding error away from an integer, so an axis
    // that should be exactly (1, 0, 0) is written back out that way.
    (
        vec::correct(quat * u_axis, 0, AXIS_CORRECT_EPSILON),
        vec::correct(quat * v_axis, 0, AXIS_CORRECT_EPSILON),
    )
}

/// The epsilon used when snapping rotated UV axes back to round values.
const AXIS_CORRECT_EPSILON: f64 = 0.001;

/// The point on a face's plane that has the given UV coordinates.
///
/// The inverse of [`UvCoordSystem::uv_coords`], which the UV editor needs: rotating or
/// scaling a material about a chosen point means knowing which point on the face that is,
/// so the offsets can be corrected to hold it still.
///
/// Returns nothing when the axes and the plane's normal do not span space, which happens
/// only for a degenerate mapping.
pub fn point_at_uv(
    system: &UvCoordSystem,
    uv: Vec2f,
    plane: &geometry_math::plane::Plane3d,
    attributes: &UvAttributes,
) -> Option<Vec3d> {
    let (u_axis, v_axis) = system.axes(plane.normal, attributes);
    let along_u = f64::from((uv.x() - attributes.offset.x()) * safe_scale(attributes.scale.x()));
    let along_v = f64::from((uv.y() - attributes.offset.y()) * safe_scale(attributes.scale.y()));

    // Three planes meet at the point: the two the coordinates name, and the face's own.
    let rows = geometry_math::mat::Matrix::<f64, 3, 3>::from_rows([
        [u_axis.x(), u_axis.y(), u_axis.z()],
        [v_axis.x(), v_axis.y(), v_axis.z()],
        [plane.normal.x(), plane.normal.y(), plane.normal.z()],
    ]);
    let inverse = geometry_math::mat::invert(rows)?;
    Some(inverse * Vec3d::new(along_u, along_v, plane.distance))
}

/// How a mapping is carried onto a face that faces a different way.
///
/// `map_model::WrapStyle`. The difference shows at an edge between two faces: projection keeps
/// the material flat and lets it foreshorten, rotation turns it so it wraps round the
/// corner without stretching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapStyle {
    /// Keep the axes as they are, turning them only by whole quarter turns.
    #[default]
    Projection,
    /// Turn the axes with the face, so the material wraps round the corner.
    Rotation,
}

/// `updateNormalWithProjection`.
///
/// Builds a cube whose front face is the mapping as it stands and whose other five are
/// quarter turns from it, then takes the face whose own normal points most nearly the way
/// the new one does. The point is to change the mapping as little as possible: this is
/// what makes a parallel system behave like a paraxial one, which only ever has six
/// orientations to choose from.
fn project_axes(u_axis: Vec3d, v_axis: Vec3d, new_normal: Vec3d) -> UvCoordSystem {
    let u = vec::normalize(u_axis);
    let v = vec::normalize(v_axis);
    let quarter_turn = |axis: Vec3d, degrees: f64| {
        geometry_math::quat::Quatd::from_axis_angle(axis, scalar::to_radians(degrees))
    };

    let mut candidates = vec![
        (u_axis, v_axis), // front
        (v_axis, u_axis), // back
    ];
    for rotation in [
        quarter_turn(u, 90.0),  // bottom
        quarter_turn(u, -90.0), // top
        quarter_turn(v, 90.0),  // left
        quarter_turn(v, -90.0), // right
    ] {
        candidates.push((rotation * u_axis, rotation * v_axis));
    }

    let best = candidates
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            let cosine = |axes: &(Vec3d, Vec3d)| {
                vec::dot(vec::normalize(vec::cross(axes.0, axes.1)), new_normal)
            };
            cosine(a)
                .partial_cmp(&cosine(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
        .unwrap_or(0);

    // Index 0 is no change at all, and index 1 is a half turn, which reads worse than
    // simply projecting the front axes; both are left alone.
    let (u_axis, v_axis) = if best >= 2 {
        candidates[best]
    } else {
        (u_axis, v_axis)
    };
    UvCoordSystem::Parallel { u_axis, v_axis }
}

/// `updateNormalWithRotation`.
///
/// Turns the axes by the same rotation that takes the old normal to the new one, so the
/// material carries round the corner unstretched.
fn rotate_axes(
    u_axis: Vec3d,
    v_axis: Vec3d,
    old_normal: Vec3d,
    new_normal: Vec3d,
) -> UvCoordSystem {
    let cross = vec::cross(old_normal, new_normal);
    if cross == Vec3d::zero() {
        // The two faces are parallel, one way round or the other, so there is nothing to
        // turn the axes by.
        return UvCoordSystem::Parallel { u_axis, v_axis };
    }

    let axis = vec::normalize(cross);
    let angle = vec::measure_angle_3d(new_normal, old_normal, axis);
    let rotation = geometry_math::quat::Quatd::from_axis_angle(axis, angle);

    UvCoordSystem::Parallel {
        u_axis: rotation * u_axis,
        v_axis: rotation * v_axis,
    }
}

/// Returns UV axes lying in the plane with the given normal.
///
/// Used to give a Valve 220 face its default mapping. The U axis is the paraxial one
/// projected into the face's plane, which keeps a face that is nearly axis-aligned
/// looking the way the Standard format would have mapped it.
pub fn parallel_axes(normal: Vec3d) -> (Vec3d, Vec3d) {
    let index = plane_normal_index(normal);
    let (base_u, _) = base_axes(index);

    // Remove the part of the base U axis that points along the normal, leaving it in the
    // face's plane.
    let u = base_u - normal * vec::dot(base_u, normal);
    let u = if vec::squared_length(u) > f64::ALMOST_ZERO {
        vec::normalize(u)
    } else {
        // The base axis was parallel to the normal, which the base table is chosen to
        // avoid; fall back to any perpendicular direction.
        vec::normalize(vec::cross(normal, Vec3d::new(0.0, 0.0, 1.0)))
    };
    let v = vec::normalize(vec::cross(normal, u));
    (u, -v)
}

/// The Quake 2 and Quake 3 surface flags a face may carry.
///
/// These follow the UV fields on a face line when the map format calls for them. The
/// editor does not interpret them; it preserves them so a round trip through the editor
/// does not lose what the compiler needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SurfaceAttributes {
    /// The content flags, describing what fills the volume behind the face.
    pub content_flags: i32,
    /// The surface flags, describing how the face itself is treated.
    pub surface_flags: i32,
    /// A format-defined value, typically the light emitted by the surface.
    pub value: i32,
}

#[cfg(test)]
mod inverse_tests {
    use super::*;
    use geometry_math::plane::Plane3d;

    fn attributes(offset: Vec2f, scale: Vec2f, rotation: f32) -> UvAttributes {
        UvAttributes {
            offset,
            scale,
            rotation,
        }
    }

    /// Every combination worth checking: both systems, at an angle, scaled and offset.
    fn cases() -> Vec<(UvCoordSystem, Plane3d, UvAttributes)> {
        let up =
            Plane3d::from_anchor_and_normal(Vec3d::new(0.0, 0.0, 32.0), Vec3d::new(0.0, 0.0, 1.0));
        let slanted = Plane3d::from_anchor_and_normal(
            Vec3d::new(8.0, 0.0, 16.0),
            vec::normalize(Vec3d::new(1.0, 2.0, 3.0)),
        );

        let mut cases = Vec::new();
        for plane in [up, slanted] {
            let (u_axis, v_axis) = parallel_axes(plane.normal);
            for system in [
                UvCoordSystem::Paraxial,
                UvCoordSystem::Parallel { u_axis, v_axis },
            ] {
                cases.push((
                    system,
                    plane,
                    attributes(Vec2f::zero(), Vec2f::new(1.0, 1.0), 0.0),
                ));
                cases.push((
                    system,
                    plane,
                    attributes(Vec2f::new(13.0, -7.0), Vec2f::new(2.0, 0.5), 30.0),
                ));
            }
        }
        cases
    }

    #[test]
    fn a_point_found_from_its_coordinates_has_those_coordinates() {
        for (system, plane, attributes) in cases() {
            for uv in [
                Vec2f::zero(),
                Vec2f::new(64.0, 0.0),
                Vec2f::new(-19.0, 43.0),
            ] {
                let point = point_at_uv(&system, uv, &plane, &attributes)
                    .expect("the mapping is not degenerate");
                let round_trip = system.uv_coords(point, plane.normal, &attributes);
                geometry_math::assert_approx_eq!(round_trip, uv, 1e-3);
            }
        }
    }

    #[test]
    fn the_point_found_lies_on_the_face() {
        for (system, plane, attributes) in cases() {
            let point = point_at_uv(&system, Vec2f::new(11.0, 23.0), &plane, &attributes)
                .expect("the mapping is not degenerate");
            // On the plane, which is what makes it a point on the face rather than
            // somewhere in space that happens to have those coordinates.
            geometry_math::assert_approx_eq!(plane.point_distance(point), 0.0, 1e-6);
        }
    }
}

#[cfg(test)]
mod wrap_tests {
    use super::*;

    fn parallel(u: Vec3d, v: Vec3d) -> UvCoordSystem {
        UvCoordSystem::Parallel {
            u_axis: u,
            v_axis: v,
        }
    }

    fn axes_of(system: UvCoordSystem) -> (Vec3d, Vec3d) {
        match system {
            UvCoordSystem::Parallel { u_axis, v_axis } => (u_axis, v_axis),
            UvCoordSystem::Paraxial => panic!("a paraxial system stores no axes"),
        }
    }

    #[test]
    fn shearing_leans_one_axis_towards_the_other() {
        // The top of a box in the Valve format: U along X, V along -Y.
        let normal = Vec3d::new(0.0, 0.0, 1.0);
        let system = UvCoordSystem::Parallel {
            u_axis: Vec3d::new(1.0, 0.0, 0.0),
            v_axis: Vec3d::new(0.0, -1.0, 0.0),
        };
        let UvCoordSystem::Parallel { u_axis, v_axis } =
            system.sheared(normal, Vec2f::new(0.5, 0.0))
        else {
            panic!("a parallel system stays parallel");
        };
        // The first factor leans V over towards U; U is left alone.
        assert!(vec::is_equal(&u_axis, &Vec3d::new(1.0, 0.0, 0.0), 1e-9));
        assert!(vec::is_equal(&v_axis, &Vec3d::new(0.5, -1.0, 0.0), 1e-9));

        let UvCoordSystem::Parallel { u_axis, v_axis } =
            system.sheared(normal, Vec2f::new(0.0, -0.25))
        else {
            panic!("a parallel system stays parallel");
        };
        assert!(vec::is_equal(&u_axis, &Vec3d::new(1.0, 0.25, 0.0), 1e-9));
        assert!(vec::is_equal(&v_axis, &Vec3d::new(0.0, -1.0, 0.0), 1e-9));
    }

    #[test]
    fn a_paraxial_system_cannot_be_sheared() {
        let system = UvCoordSystem::Paraxial;
        assert_eq!(
            system.sheared(Vec3d::new(0.0, 0.0, 1.0), Vec2f::new(0.5, 0.5)),
            UvCoordSystem::Paraxial
        );
    }

    #[test]
    fn a_skew_undone_by_its_inverse_gives_the_axes_back() {
        let normal = vec::normalize(Vec3d::new(1.0, 1.0, 1.0));
        let system = UvCoordSystem::Parallel {
            u_axis: vec::normalize(Vec3d::new(1.0, -1.0, 0.0)),
            v_axis: vec::normalize(Vec3d::new(1.0, 1.0, -2.0)),
        };
        let (a, b) = (0.3, -0.6);
        let det = 1.0 - a * b;
        let sheared = system.sheared(normal, Vec2f::new(a as f32, b as f32));
        let back = sheared.skewed(normal, [[1.0 / det, -a / det], [-b / det, 1.0 / det]]);
        let (
            UvCoordSystem::Parallel { u_axis, v_axis },
            UvCoordSystem::Parallel {
                u_axis: u0,
                v_axis: v0,
            },
        ) = (back, system)
        else {
            panic!("parallel systems");
        };
        assert!(vec::is_equal(&u_axis, &u0, 1e-6));
        assert!(vec::is_equal(&v_axis, &v0, 1e-6));
    }

    #[test]
    fn a_paraxial_system_has_nothing_to_carry_across() {
        // Its axes come from whichever coordinate plane the face most faces, so a copy is
        // already correct for the target.
        let system = UvCoordSystem::Paraxial;
        for style in [WrapStyle::Projection, WrapStyle::Rotation] {
            assert_eq!(
                system.updated_for_normal(
                    Vec3d::new(0.0, 0.0, 1.0),
                    Vec3d::new(1.0, 0.0, 0.0),
                    style
                ),
                UvCoordSystem::Paraxial
            );
        }
    }

    #[test]
    fn a_face_that_has_not_turned_keeps_its_axes() {
        let system = parallel(Vec3d::new(1.0, 0.0, 0.0), Vec3d::new(0.0, -1.0, 0.0));
        let up = Vec3d::new(0.0, 0.0, 1.0);

        for style in [WrapStyle::Projection, WrapStyle::Rotation] {
            assert_eq!(
                system.updated_for_normal(up, up, style),
                system,
                "{style:?}"
            );
        }
    }

    #[test]
    fn rotation_turns_the_axes_with_the_face() {
        // A quarter turn about X, from facing up to facing along -Y.
        let system = parallel(Vec3d::new(1.0, 0.0, 0.0), Vec3d::new(0.0, -1.0, 0.0));
        let from = Vec3d::new(0.0, 0.0, 1.0);
        let to = Vec3d::new(0.0, -1.0, 0.0);

        let (u, v) = axes_of(system.updated_for_normal(from, to, WrapStyle::Rotation));
        // The axes stay perpendicular to each other and now lie in the new face's plane.
        geometry_math::assert_approx_eq!(vec::dot(u, v), 0.0, 1e-9);
        geometry_math::assert_approx_eq!(vec::dot(u, to), 0.0, 1e-9);
        geometry_math::assert_approx_eq!(vec::dot(v, to), 0.0, 1e-9);
        // U ran along X and the turn was about X, so it did not move.
        geometry_math::assert_approx_eq!(u, Vec3d::new(1.0, 0.0, 0.0), 1e-9);
    }

    #[test]
    fn rotation_leaves_a_parallel_face_alone() {
        // Facing the opposite way is still parallel, so there is no axis to turn about.
        let system = parallel(Vec3d::new(1.0, 0.0, 0.0), Vec3d::new(0.0, -1.0, 0.0));
        let up = Vec3d::new(0.0, 0.0, 1.0);
        assert_eq!(
            system.updated_for_normal(up, -up, WrapStyle::Rotation),
            system
        );
    }

    #[test]
    fn projection_picks_the_quarter_turn_that_faces_the_new_way() {
        let system = parallel(Vec3d::new(1.0, 0.0, 0.0), Vec3d::new(0.0, -1.0, 0.0));
        let from = Vec3d::new(0.0, 0.0, 1.0);
        let to = Vec3d::new(0.0, -1.0, 0.0);

        let (u, v) = axes_of(system.updated_for_normal(from, to, WrapStyle::Projection));
        // Whatever it picked, the axes it produces face the new way.
        let facing = vec::normalize(vec::cross(u, v));
        geometry_math::assert_approx_eq!(vec::dot(facing, to), 1.0, 1e-9);
    }

    #[test]
    fn projection_only_ever_makes_quarter_turns() {
        // Every axis it produces is one of the original two, one way round or the other,
        // which is what keeps it behaving like a paraxial system.
        let system = parallel(Vec3d::new(1.0, 0.0, 0.0), Vec3d::new(0.0, -1.0, 0.0));
        let from = Vec3d::new(0.0, 0.0, 1.0);

        for to in [
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(-1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
            Vec3d::new(0.0, -1.0, 0.0),
            Vec3d::new(0.0, 0.0, -1.0),
        ] {
            let (u, v) = axes_of(system.updated_for_normal(from, to, WrapStyle::Projection));
            for axis in [u, v] {
                let on_an_axis = (0..3)
                    .filter(|component| axis[*component].abs() > 1e-9)
                    .count();
                assert_eq!(on_an_axis, 1, "{axis:?} is not a quarter turn away");
                geometry_math::assert_approx_eq!(vec::length(axis), 1.0, 1e-9);
            }
        }
    }

    #[test]
    fn a_face_at_an_angle_keeps_its_material_unstretched_under_rotation() {
        // Rotation is the style that carries a material round a corner, so the axes it
        // gives keep their length: the material is turned, not squashed.
        let system = parallel(Vec3d::new(1.0, 0.0, 0.0), Vec3d::new(0.0, -1.0, 0.0));
        let from = Vec3d::new(0.0, 0.0, 1.0);
        let to = vec::normalize(Vec3d::new(1.0, 1.0, 1.0));

        let (u, v) = axes_of(system.updated_for_normal(from, to, WrapStyle::Rotation));
        geometry_math::assert_approx_eq!(vec::length(u), 1.0, 1e-9);
        geometry_math::assert_approx_eq!(vec::length(v), 1.0, 1e-9);
        geometry_math::assert_approx_eq!(vec::dot(u, to), 0.0, 1e-9);
    }

    #[test]
    fn the_primitive_axis_base_matches_computeaxisbase_for_the_six_walls() {
        // `ComputeAxisBase` from the Quake 3 tool chain, tabulated for each axis normal.
        let cases = [
            ((0.0, 0.0, 1.0), (0.0, 1.0, 0.0), (1.0, 0.0, 0.0)), // floor
            ((0.0, 0.0, -1.0), (0.0, 1.0, 0.0), (-1.0, 0.0, 0.0)), // ceiling
            ((1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, -1.0)), // +X wall
            ((-1.0, 0.0, 0.0), (0.0, -1.0, 0.0), (0.0, 0.0, -1.0)),
            ((0.0, 1.0, 0.0), (-1.0, 0.0, 0.0), (0.0, 0.0, -1.0)), // +Y wall
            ((0.0, -1.0, 0.0), (1.0, 0.0, 0.0), (0.0, 0.0, -1.0)),
        ];
        for ((nx, ny, nz), (sx, sy, sz), (tx, ty, tz)) in cases {
            let (s, t) = primitive_axis_base(Vec3d::new(nx, ny, nz));
            geometry_math::assert_approx_eq!(s, Vec3d::new(sx, sy, sz), 1e-9);
            geometry_math::assert_approx_eq!(t, Vec3d::new(tx, ty, tz), 1e-9);
        }
    }

    #[test]
    fn a_primitive_matrix_scaled_by_two_halves_the_uv_step() {
        // A matrix with twice the gradient (`2/128` rather than `1/128`) means the texture
        // repeats twice as often, so the UV axes come out twice as long.
        let normal = Vec3d::new(0.0, 0.0, 1.0);
        let one = parallel_from_primitive_matrix(
            [[1.0 / 128.0, 0.0, 0.0], [0.0, 1.0 / 128.0, 0.0]],
            normal,
            Vec2f::new(128.0, 128.0),
        );
        let two = parallel_from_primitive_matrix(
            [[2.0 / 128.0, 0.0, 0.0], [0.0, 2.0 / 128.0, 0.0]],
            normal,
            Vec2f::new(128.0, 128.0),
        );
        let (UvCoordSystem::Parallel { u_axis: u1, .. }, _) = one else {
            panic!()
        };
        let (UvCoordSystem::Parallel { u_axis: u2, .. }, _) = two else {
            panic!()
        };
        geometry_math::assert_approx_eq!(u2, u1 * 2.0, 1e-9);
    }
}
