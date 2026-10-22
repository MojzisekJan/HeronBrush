//! A single face of a brush, as it appears in a `.map` file.
//!
//! A face is three points defining its plane, the material on it, and how that material
//! is mapped. The plane's normal points out of the brush; the three points are given
//! clockwise as seen from outside, which is the convention
//! [`geometry_math::plane::from_points`] and [`crate::polyhedron`] both use.

use geometry_math::plane::Plane3d;
use geometry_math::scalar;
use geometry_math::vec::{Vec2f, Vec3d};
use geometry_math::{plane, vec};

use crate::uv::{SurfaceAttributes, UvAttributes, UvCoordSystem};

/// The material name used for a face with no material assigned.
pub const NO_MATERIAL_NAME: &str = "__TB_empty";

/// The name of the material on a face.
///
/// A map has thousands of faces and a handful of distinct materials, so the name is an
/// `Arc<str>` and the reader hands every face that names `"stone"` the same one. It reads
/// as a `str` through `Deref`, so most call sites do not notice. It serializes as a plain
/// string, so the collab wire format needs no `rc` feature (deserialized copies are not
/// shared, which does not matter for a stream of independent faces).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialName(std::sync::Arc<str>);

impl MaterialName {
    /// The name as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for MaterialName {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MaterialName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for MaterialName {
    fn from(name: &str) -> Self {
        MaterialName(std::sync::Arc::from(name))
    }
}

impl From<String> for MaterialName {
    fn from(name: String) -> Self {
        MaterialName(std::sync::Arc::from(name))
    }
}

impl From<MaterialName> for String {
    fn from(name: MaterialName) -> Self {
        name.0.to_string()
    }
}

impl AsRef<str> for MaterialName {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for MaterialName {
    #[inline]
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for MaterialName {
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<&str> for MaterialName {
    fn eq(&self, other: &&str) -> bool {
        &*self.0 == *other
    }
}

impl PartialEq<MaterialName> for str {
    fn eq(&self, other: &MaterialName) -> bool {
        self == &*other.0
    }
}

impl PartialEq<String> for MaterialName {
    fn eq(&self, other: &String) -> bool {
        &*self.0 == other.as_str()
    }
}

impl PartialEq<MaterialName> for String {
    fn eq(&self, other: &MaterialName) -> bool {
        self.as_str() == &*other.0
    }
}

impl Default for MaterialName {
    fn default() -> Self {
        MaterialName::from("")
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for MaterialName {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for MaterialName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(MaterialName::from(String::deserialize(deserializer)?))
    }
}

/// A colour, as a `.map` file writes one: three whole numbers from 0 to 255.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rgb {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
}

impl Rgb {
    /// A colour from its three components.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// The colour as the three 0-to-1 floats a renderer wants.
    pub fn to_f32(self) -> [f32; 3] {
        [
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
        ]
    }

    /// The nearest colour to three 0-to-1 floats.
    pub fn from_f32(rgb: [f32; 3]) -> Self {
        let round = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        Self::new(round(rgb[0]), round(rgb[1]), round(rgb[2]))
    }
}

/// A colour and its position in a gradient, from zero to one.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColorStop {
    /// Normalized position in the transition.
    pub position: f32,
    /// Colour at this position.
    pub color: Rgb,
}

/// Two to eight sorted colour stops. Fixed storage keeps face paint inexpensive to copy.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GradientStops {
    stops: [ColorStop; 8],
    len: u8,
}
impl GradientStops {
    /// Validate and sort two to eight stops.
    pub fn new(stops: &[ColorStop]) -> Option<Self> {
        if !(2..=8).contains(&stops.len())
            || stops
                .iter()
                .any(|s| !s.position.is_finite() || !(0.0..=1.0).contains(&s.position))
        {
            return None;
        }
        let mut result = Self {
            stops: [ColorStop {
                position: 0.0,
                color: Rgb::new(0, 0, 0),
            }; 8],
            len: stops.len() as u8,
        };
        result.stops[..stops.len()].copy_from_slice(stops);
        result.stops[..stops.len()].sort_by(|a, b| a.position.total_cmp(&b.position));
        Some(result)
    }
    /// Stops in ascending position order.
    pub fn as_slice(&self) -> &[ColorStop] {
        &self.stops[..usize::from(self.len).clamp(2, 8)]
    }
    /// Sample the piecewise linear colour ramp, clamping outside its ends.
    pub fn at(&self, t: f32) -> [f32; 3] {
        let stops = self.as_slice();
        if t < stops[0].position {
            return stops[0].color.to_f32();
        }
        for pair in stops.windows(2) {
            if t < pair[1].position {
                let t = ((t - pair[0].position)
                    / (pair[1].position - pair[0].position).max(f32::EPSILON))
                .clamp(0.0, 1.0);
                let a = pair[0].color.to_f32();
                let b = pair[1].color.to_f32();
                return std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t);
            }
        }
        stops.last().unwrap().color.to_f32()
    }
}

/// Where a gradient runs on its face: two points given as fractions of the face's UV bounds,
/// (0, 0) at one corner of the bounds and (1, 1) at the other, so they keep their place on
/// the face as it moves, grows or turns. A linear gradient runs from `start` (position 0) to
/// `end` (position 1), its colours constant across that line; a radial one is centred on
/// `start` and reaches position 1 at `end`. Points beyond the bounds are allowed.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GradientEnds {
    /// Position 0: the start of a linear gradient, the centre of a radial one.
    pub start: [f32; 2],
    /// Position 1.
    pub end: [f32; 2],
}

/// A colour painted on a face, instead of the material it wears.
///
/// Face colours are stored as comments after the standard face syntax. Readers can ignore them
/// without rejecting the map; tools that discard comments also discard the colours.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FaceColor {
    /// A linear or centred radial gradient with independently positioned colour stops.
    MultiGradient {
        /// Ordered colour stops.
        stops: GradientStops,
        /// True for an ellipse centred in the face UV bounds.
        radial: bool,
        /// Linear direction in degrees from the U axis; unused for radial paint, and for
        /// a gradient with `ends`.
        angle: f32,
        /// Opacity, zero to 255.
        alpha: u8,
        /// Where the gradient runs on the face; `None` fits it to the face: a linear one
        /// across the face along `angle`, a radial one as an ellipse filling the bounds.
        ends: Option<GradientEnds>,
    },
    /// One colour across the whole face.
    Solid {
        /// The colour.
        color: Rgb,
        /// How opaque the face is drawn, 0 clear to 255 solid. A face with any material
        /// under it shows that material through where this is not 255.
        alpha: u8,
    },
    /// A blend from one colour to another across the face.
    Gradient {
        /// The colour at the start.
        from: Rgb,
        /// The colour at the end.
        to: Rgb,
        /// Which way the blend runs, in degrees anticlockwise from the face's U axis.
        angle: f32,
        /// How opaque the face is drawn, 0 clear to 255 solid.
        alpha: u8,
    },
}

impl FaceColor {
    /// An opaque solid colour.
    pub const fn solid(color: Rgb) -> Self {
        Self::Solid { color, alpha: 255 }
    }

    /// An opaque gradient.
    pub const fn gradient(from: Rgb, to: Rgb, angle: f32) -> Self {
        Self::Gradient {
            from,
            to,
            angle,
            alpha: 255,
        }
    }

    /// How opaque the face is drawn, 0 to 255.
    pub const fn alpha(self) -> u8 {
        match self {
            Self::Solid { alpha, .. }
            | Self::Gradient { alpha, .. }
            | Self::MultiGradient { alpha, .. } => alpha,
        }
    }

    /// The same colour at a different opacity.
    pub const fn with_alpha(self, alpha: u8) -> Self {
        match self {
            Self::MultiGradient {
                stops,
                radial,
                angle,
                ends,
                ..
            } => Self::MultiGradient {
                stops,
                radial,
                angle,
                alpha,
                ends,
            },
            Self::Solid { color, .. } => Self::Solid { color, alpha },
            Self::Gradient {
                from, to, angle, ..
            } => Self::Gradient {
                from,
                to,
                angle,
                alpha,
            },
        }
    }
}

/// Shared UV-space evaluator for editor rendering, thumbnails and texture export.
pub struct PaintOnFace {
    paint: FaceColor,
    lo: Vec2f,
    span: Vec2f,
    direction: Vec2f,
    start: f32,
    extent: f32,
    /// A gradient's own ends, in UV.
    ends: Option<(Vec2f, Vec2f)>,
}
impl PaintOnFace {
    /// Fit paint to the face UV bounds.
    pub fn new(face: &BrushFace, polygon: &[Vec3d], paint: FaceColor) -> Self {
        Self::from_uv(
            &polygon
                .iter()
                .map(|p| face.uv_coords(*p))
                .collect::<Vec<_>>(),
            paint,
        )
    }
    /// Fit paint to an already projected polygon.
    pub fn from_uv(polygon: &[Vec2f], paint: FaceColor) -> Self {
        let angle = match paint {
            FaceColor::Gradient { angle, .. } | FaceColor::MultiGradient { angle, .. } => angle,
            _ => 0.0,
        }
        .to_radians();
        let direction = Vec2f::new(angle.cos(), angle.sin());
        let mut lo = Vec2f::new(f32::INFINITY, f32::INFINITY);
        let mut hi = Vec2f::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        let (mut start, mut end) = (f32::INFINITY, f32::NEG_INFINITY);
        for p in polygon {
            lo = Vec2f::new(lo.x().min(p.x()), lo.y().min(p.y()));
            hi = Vec2f::new(hi.x().max(p.x()), hi.y().max(p.y()));
            let d = vec::dot(*p, direction);
            start = start.min(d);
            end = end.max(d);
        }
        if polygon.is_empty() {
            lo = Vec2f::new(0.0, 0.0);
            hi = Vec2f::new(1.0, 1.0);
            start = 0.0;
            end = 1.0;
        }
        let mut sampler = Self {
            paint,
            lo,
            span: Vec2f::new((hi.x() - lo.x()).max(1e-6), (hi.y() - lo.y()).max(1e-6)),
            direction,
            start,
            extent: (end - start).max(1e-6),
            ends: None,
        };
        if let FaceColor::MultiGradient {
            ends: Some(ends), ..
        } = paint
        {
            let (a, b) = (
                sampler.from_bounds(ends.start),
                sampler.from_bounds(ends.end),
            );
            // Two ends in one place have no direction: the gradient keeps its fitted one.
            if vec::length(b - a) > 1e-6 {
                sampler.ends = Some((a, b));
            }
        }
        sampler
    }
    /// The UV point at `fraction` of the face's UV bounds.
    pub fn from_bounds(&self, fraction: [f32; 2]) -> Vec2f {
        Vec2f::new(
            self.lo.x() + fraction[0] * self.span.x(),
            self.lo.y() + fraction[1] * self.span.y(),
        )
    }
    /// Where `uv` falls in the face's UV bounds, as [`GradientEnds`] holds a point.
    pub fn to_bounds(&self, uv: Vec2f) -> [f32; 2] {
        [
            (uv.x() - self.lo.x()) / self.span.x(),
            (uv.y() - self.lo.y()) / self.span.y(),
        ]
    }
    /// Where the gradient runs, as [`GradientEnds`]: its own ends, or for a gradient fitted
    /// to the face the ends that paint the same (a radial one as a circle as wide as the
    /// bounds). `None` for paint that is not a gradient.
    pub fn ends(&self) -> Option<GradientEnds> {
        if !matches!(
            self.paint,
            FaceColor::MultiGradient { .. } | FaceColor::Gradient { .. }
        ) {
            return None;
        }
        let (a, b) = self.ends_uv();
        Some(GradientEnds {
            start: self.to_bounds(a),
            end: self.to_bounds(b),
        })
    }
    /// The gradient's two ends in UV, its own or the fitted ones.
    pub fn ends_uv(&self) -> (Vec2f, Vec2f) {
        if let Some(ends) = self.ends {
            return ends;
        }
        let centre = self.from_bounds([0.5, 0.5]);
        if matches!(self.paint, FaceColor::MultiGradient { radial: true, .. }) {
            return (centre, centre + Vec2f::new(self.span.x() / 2.0, 0.0));
        }
        let across = vec::dot(centre, self.direction);
        (
            centre + self.direction * (self.start - across),
            centre + self.direction * (self.start + self.extent - across),
        )
    }
    /// The gradient position at `uv`, unclamped: 0 at the start, 1 at the end.
    pub fn position_at_uv(&self, uv: Vec2f) -> f32 {
        let radial = matches!(self.paint, FaceColor::MultiGradient { radial: true, .. });
        match self.ends {
            Some((a, b)) if radial => vec::length(uv - a) / vec::length(b - a),
            Some((a, b)) => vec::dot(uv - a, b - a) / vec::dot(b - a, b - a),
            None if radial => {
                let x = (uv.x() - self.lo.x()) / self.span.x() * 2.0 - 1.0;
                let y = (uv.y() - self.lo.y()) / self.span.y() * 2.0 - 1.0;
                (x * x + y * y).sqrt()
            }
            None => (vec::dot(uv, self.direction) - self.start) / self.extent,
        }
    }
    /// Sample paint at UV coordinates.
    pub fn at_uv(&self, uv: Vec2f) -> [f32; 3] {
        let t = self.position_at_uv(uv).clamp(0.0, 1.0);
        match self.paint {
            FaceColor::Solid { color, .. } => color.to_f32(),
            FaceColor::MultiGradient { stops, .. } => stops.at(t),
            FaceColor::Gradient { from, to, .. } => {
                let a = from.to_f32();
                let b = to.to_f32();
                std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
            }
        }
    }
    /// Sample paint at a world position.
    pub fn at(&self, face: &BrushFace, point: Vec3d) -> [f32; 3] {
        self.at_uv(face.uv_coords(point))
    }
}

/// Subdivide painted faces so interior stops and radial centres survive vertex-colour rendering.
pub fn paint_triangles(
    face: &BrushFace,
    polygon: &[Vec3d],
    paint: Option<FaceColor>,
) -> Vec<[Vec3d; 3]> {
    if let Some(
        paint @ FaceColor::MultiGradient {
            stops,
            radial: false,
            ..
        },
    ) = paint
    {
        let sampler = PaintOnFace::new(face, polygon, paint);
        let along = |p| sampler.position_at_uv(face.uv_coords(p));
        let clip = |input: &[Vec3d], boundary: f32, above: bool| {
            let mut out = Vec::new();
            for i in 0..input.len() {
                let a = input[i];
                let b = input[(i + 1) % input.len()];
                let da = along(a) - boundary;
                let db = along(b) - boundary;
                let inside_a = if above { da >= 0.0 } else { da <= 0.0 };
                let inside_b = if above { db >= 0.0 } else { db <= 0.0 };
                if inside_a {
                    out.push(a);
                }
                if inside_a != inside_b {
                    out.push(a + (b - a) * f64::from(da / (da - db)));
                }
            }
            out
        };
        // Placed ends may leave part of the face before the start or past the end; those
        // parts take the end colours, in bands of their own.
        let mut boundaries = vec![-1e30, 0.0];
        boundaries.extend(stops.as_slice().iter().map(|s| s.position));
        boundaries.extend([1.0, 1e30]);
        boundaries.dedup();
        let mut triangles = Vec::new();
        for band in boundaries.windows(2) {
            let part = clip(&clip(polygon, band[0], true), band[1], false);
            for i in 1..part.len().saturating_sub(1) {
                triangles.push([part[0], part[i], part[i + 1]]);
            }
        }
        return triangles;
    }
    let steps = if matches!(paint, Some(FaceColor::MultiGradient { radial: true, .. })) {
        16
    } else {
        1
    };
    let mut triangles = Vec::new();
    for k in 1..polygon.len().saturating_sub(1) {
        let a = polygon[0];
        let b = polygon[k];
        let c = polygon[k + 1];
        let point = |i: usize, j: usize| {
            a + (b - a) * (i as f64 / steps as f64) + (c - a) * (j as f64 / steps as f64)
        };
        for i in 0..steps {
            for j in 0..steps - i {
                triangles.push([point(i, j), point(i + 1, j), point(i, j + 1)]);
                if i + j + 1 < steps {
                    triangles.push([point(i + 1, j), point(i + 1, j + 1), point(i, j + 1)]);
                }
            }
        }
    }
    triangles
}

/// A gradient worked out for one face, ready to give a colour to any point on it.
///
/// A face has no natural axes of its own, so the blend runs along the face's own U axis,
/// turned by the angle asked for. How far across the face a point is comes from the span
/// the face covers in that direction, which is what makes a gradient look the same
/// whatever the face's size or the scale the material is at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientOnFace {
    /// The direction the blend runs, in the face's UV space.
    direction: Vec2f,
    /// Where the blend starts, along that direction.
    start: f32,
    /// How far it runs. Never zero: a face with no extent gets one, so that dividing by
    /// it gives the colour at the start rather than nothing at all.
    span: f32,
    /// The colour at the start.
    from: Rgb,
    /// The colour at the end.
    to: Rgb,
}

impl GradientOnFace {
    /// Works out how a gradient falls across the given face polygon.
    pub fn new(face: &BrushFace, polygon: &[Vec3d], from: Rgb, to: Rgb, angle: f32) -> Self {
        let radians = angle.to_radians();
        let direction = Vec2f::new(radians.cos(), radians.sin());

        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for corner in polygon {
            let along = vec::dot(face.uv_coords(*corner), direction);
            min = min.min(along);
            max = max.max(along);
        }

        let span = max - min;
        Self {
            direction,
            start: if min.is_finite() { min } else { 0.0 },
            span: if span.is_finite() && span > f32::EPSILON {
                span
            } else {
                1.0
            },
            from,
            to,
        }
    }

    /// The colour at a point on the face.
    pub fn at(&self, face: &BrushFace, point: Vec3d) -> [f32; 3] {
        let along = vec::dot(face.uv_coords(point), self.direction);
        let t = ((along - self.start) / self.span).clamp(0.0, 1.0);
        let from = self.from.to_f32();
        let to = self.to.to_f32();
        [
            from[0] + (to[0] - from[0]) * t,
            from[1] + (to[1] - from[1]) * t,
            from[2] + (to[2] - from[2]) * t,
        ]
    }
}

/// Per-face PBR factors, stored as percentages for stable material deduplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SurfaceMaterial {
    /// Roughness percentage, from glossy to matte.
    pub roughness: u8,
    /// Metallic percentage.
    pub metallic: u8,
    /// Emission intensity in hundredths, 0..800. Uses the albedo colour.
    #[cfg_attr(feature = "serde", serde(default))]
    pub emission: u16,
}
impl Default for SurfaceMaterial {
    fn default() -> Self {
        Self {
            roughness: 65,
            metallic: 0,
            emission: 0,
        }
    }
}
impl SurfaceMaterial {
    /// Clamped roughness, metallic and emission factors for rendering/export.
    pub fn factors(self) -> [f32; 3] {
        [
            self.roughness.min(100) as f32 / 100.0,
            self.metallic.min(100) as f32 / 100.0,
            self.emission.min(800) as f32 / 100.0,
        ]
    }
}

/// A face of a brush.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BrushFace {
    /// The three points defining the face's plane, clockwise as seen from outside.
    ///
    /// These are kept as written rather than recomputed, so that a map file loaded and
    /// saved again is unchanged where it did not need to change.
    pub points: [Vec3d; 3],
    /// The face's plane, derived from its points.
    pub plane: Plane3d,
    /// The name of the material on this face.
    pub material_name: MaterialName,
    /// How the material's axes are determined.
    pub uv_coord_system: UvCoordSystem,
    /// The offset, scale and rotation applied on top of those axes.
    pub uv_attributes: UvAttributes,
    /// The Quake 2 and Quake 3 surface flags, when the map format carries them.
    pub surface_attributes: Option<SurfaceAttributes>,
    /// A colour painted on the face, drawn instead of its material.
    ///
    /// Boxed for the same reason as [`Self::primitive_uv`]: a gradient carries eight
    /// stops inline, 76 bytes that most faces of most maps never use.
    /// [`Self::paint`] and [`Self::set_paint`] read and write it by value.
    pub color: Option<Box<FaceColor>>,
    #[cfg_attr(feature = "serde", serde(default))]
    /// Optional PBR override, independent of paint or texture.
    pub surface_material: Option<SurfaceMaterial>,
    /// Keep the plane for brush geometry and collisions, but omit its render polygon.
    #[cfg_attr(feature = "serde", serde(default))]
    pub export_hidden: bool,
    /// The unconverted brush-primitive texture matrix, for a face read from a `brushDef`
    /// or `brushDef3` block whose material was not loaded yet.
    ///
    /// It is `[[a, b, c], [d, e, f]]`, the 2x3 matrix Quake 3 and Doom 3 write instead of
    /// an offset/scale/rotation. Turning it into UV axes needs the material's pixel size,
    /// so it is kept here until [`Self::resolve_primitive_uv`] is given one; the face
    /// meanwhile wears the default paraxial alignment. `None` for every other face.
    ///
    /// Boxed because it is `None` on every face of every ordinary map, and inline it
    /// would be the largest field here: 56 bytes of a 288-byte face, held forty times
    /// over by a map with a hundred thousand faces. Behind a `Box` the absent case is
    /// eight, and the rare present one pays an allocation it is worth.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub primitive_uv: Option<Box<[[f64; 3]; 2]>>,
    /// The line in the source file this face was read from, for diagnostics.
    pub line: usize,
}

/// Two faces are the same face when they describe the same surface, whatever line of
/// whatever file each was read from.
///
/// `line` is where a face came from, for a diagnostic to point at, and not part of what
/// it is. Deriving equality over it made a face compare unequal to itself for having
/// moved down the file, which is what happens when anything at all is added above it --
/// and a three-way merge, which asks "is this the brush that was here before?", then
/// answered no. Two people editing different brushes conflicted, and one of the two
/// edits was dropped, as soon as either file had grown a line.
impl PartialEq for BrushFace {
    fn eq(&self, other: &Self) -> bool {
        self.points == other.points
            && self.plane == other.plane
            && self.material_name == other.material_name
            && self.uv_coord_system == other.uv_coord_system
            && self.uv_attributes == other.uv_attributes
            && self.surface_attributes == other.surface_attributes
            && self.color == other.color
            && self.surface_material == other.surface_material
            && self.export_hidden == other.export_hidden
            && self.primitive_uv == other.primitive_uv
    }
}

impl BrushFace {
    /// The colour painted on the face, if any.
    #[inline]
    pub fn paint(&self) -> Option<FaceColor> {
        self.color.as_deref().copied()
    }

    /// Paints the face, or clears its paint with `None`.
    pub fn set_paint(&mut self, color: Option<FaceColor>) {
        self.color = color.map(Box::new);
    }

    /// Turns a pending brush-primitive texture matrix into real UV axes, now that the
    /// material's pixel size is known.
    ///
    /// The matrix maps a point's coordinates in the face's own texture basis (Quake 3's
    /// `ComputeAxisBase`) to normalised texture coordinates; this bakes the basis and the
    /// pixel size into a [`UvCoordSystem::Parallel`] pair, which is how the rest of the
    /// editor already understands free UV axes. Does nothing for a face with no pending
    /// matrix.
    pub fn resolve_primitive_uv(&mut self, material_size: Vec2f) {
        let Some(matrix) = self.primitive_uv.take() else {
            return;
        };
        let (system, attributes) =
            crate::uv::parallel_from_primitive_matrix(*matrix, self.normal(), material_size);
        self.uv_coord_system = system;
        self.uv_attributes = attributes;
    }
}

/// Why a face could not be created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrushFaceError {
    /// The three points are colinear or coincident, so they define no plane.
    PointsDoNotDefinePlane,
}

impl std::fmt::Display for BrushFaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrushFaceError::PointsDoNotDefinePlane => {
                f.write_str("the three points of the face do not define a plane")
            }
        }
    }
}

impl std::error::Error for BrushFaceError {}

impl BrushFace {
    /// Creates a face from the three points on its plane.
    ///
    /// The points must be given clockwise as seen from outside the brush.
    pub fn from_points(
        points: [Vec3d; 3],
        material_name: impl Into<MaterialName>,
        uv_coord_system: UvCoordSystem,
        uv_attributes: UvAttributes,
    ) -> Result<Self, BrushFaceError> {
        let plane = plane::from_points(points[0], points[1], points[2])
            .ok_or(BrushFaceError::PointsDoNotDefinePlane)?;

        Ok(Self {
            points,
            plane,
            material_name: material_name.into(),
            uv_coord_system,
            uv_attributes,
            surface_attributes: None,
            color: None,
            surface_material: None,
            export_hidden: false,
            primitive_uv: None,
            line: 0,
        })
    }

    /// Returns the face's outward normal.
    #[inline]
    pub fn normal(&self) -> Vec3d {
        self.plane.normal
    }

    /// Returns the UV coordinates of the given point in texels.
    #[inline]
    pub fn uv_coords(&self, point: Vec3d) -> geometry_math::vec::Vec2f {
        self.uv_coord_system
            .uv_coords(point, self.normal(), &self.uv_attributes)
    }

    /// Carries UV axes onto this plane by rotating around a shared edge.
    /// `anchor` must lie on both planes; its texel coordinates remain unchanged.
    pub fn wrap_uv_from(&mut self, source: &Self, anchor: Vec3d) {
        let (u_axis, v_axis) = source
            .uv_coord_system
            .axes(source.normal(), &source.uv_attributes);
        self.uv_attributes = source.uv_attributes;
        self.primitive_uv = None;
        self.uv_coord_system = crate::uv::UvCoordSystem::Parallel { u_axis, v_axis }
            .updated_for_normal(
                source.normal(),
                self.normal(),
                crate::uv::WrapStyle::Rotation,
            );
        self.uv_attributes.offset += source.uv_coords(anchor) - self.uv_coords(anchor);
    }

    /* ========== UV editing ========== */

    /// Shifts the material across the face by the given number of texels.
    pub fn move_uv(&mut self, delta: Vec2f) {
        self.uv_attributes.offset += delta;
    }

    /// Turns the material on the face by the given angle in degrees.
    pub fn rotate_uv(&mut self, degrees: f32) {
        self.uv_attributes.rotation =
            scalar::normalize_degrees(self.uv_attributes.rotation + degrees);
        self.uv_coord_system = self.uv_coord_system.rotated(self.normal(), degrees);
    }

    /// Multiplies the material's scale by the given factors.
    ///
    /// A factor of zero is ignored on that axis, since a zero scale has no meaning and
    /// would make the mapping undefined.
    pub fn scale_uv(&mut self, factors: Vec2f) {
        let scale = self.uv_attributes.scale;
        self.uv_attributes.scale = Vec2f::new(
            if factors.x() == 0.0 {
                scale.x()
            } else {
                scale.x() * factors.x()
            },
            if factors.y() == 0.0 {
                scale.y()
            } else {
                scale.y() * factors.y()
            },
        );
    }

    /// Skews the material, `BrushFace::shearUv`. Only a Valve face can hold a shear; a
    /// Standard face is left as it is.
    pub fn shear_uv(&mut self, factors: Vec2f) {
        self.uv_coord_system = self.uv_coord_system.sheared(self.normal(), factors);
    }

    /// Skews the material about a point on the face, keeping that point still.
    ///
    /// `UvShearTool`'s drag, with the same offset correction as the rotation.
    pub fn shear_uv_about(&mut self, origin_uv: Vec2f, factors: Vec2f) {
        self.change_uv_about(origin_uv, |face| face.shear_uv(factors));
    }

    /// Applies a 2x2 change to the mapping's axes about a point on the face, keeping
    /// that point still. See [`UvCoordSystem::skewed`] for why a drag needs this rather
    /// than [`Self::shear_uv_about`].
    pub fn skew_uv_about(&mut self, origin_uv: Vec2f, rows: [[f64; 2]; 2]) {
        self.change_uv_about(origin_uv, |face| {
            face.uv_coord_system = face.uv_coord_system.skewed(face.normal(), rows);
        });
    }

    /// Turns the material about a point on the face, keeping that point still.
    ///
    /// After rotating UVs, adjusts the offset to preserve the chosen invariant point, so the
    /// material rotates about the handle rather than the mapping origin.
    pub fn rotate_uv_about(&mut self, origin_uv: Vec2f, degrees: f32) {
        self.change_uv_about(origin_uv, |face| face.rotate_uv(degrees));
    }

    /// Scales the material about a point on the face, keeping that point still.
    ///
    /// `UvScaleTool`'s drag, with the same offset correction.
    pub fn scale_uv_about(&mut self, origin_uv: Vec2f, factors: Vec2f) {
        self.change_uv_about(origin_uv, |face| face.scale_uv(factors));
    }

    /// Applies a change to the mapping and moves the offsets so the given point stays put.
    fn change_uv_about(&mut self, origin_uv: Vec2f, change: impl FnOnce(&mut Self)) {
        // Which point of the face is under the handle now.
        let anchor = crate::uv::point_at_uv(
            &self.uv_coord_system,
            origin_uv,
            &self.plane,
            &self.uv_attributes,
        );

        change(self);

        // And where it has ended up, which is how far the offsets have to come back.
        if let Some(anchor) = anchor {
            let moved = self.uv_coords(anchor);
            self.uv_attributes.offset += origin_uv - moved;
        }
    }

    /// Mirrors the material along one of its axes.
    pub fn flip_uv(&mut self, horizontally: bool) {
        let scale = self.uv_attributes.scale;
        self.uv_attributes.scale = if horizontally {
            Vec2f::new(-scale.x(), scale.y())
        } else {
            Vec2f::new(scale.x(), -scale.y())
        };
    }

    /// Returns the material to its default alignment on this face, keeping the kind of UV
    /// coordinate system the face has: a Valve face's free axes are re-derived from the
    /// normal but stay free.
    pub fn reset_uv(&mut self) {
        self.primitive_uv = None;
        self.uv_attributes = UvAttributes::default();
        self.uv_coord_system = self.uv_coord_system.reset(self.normal());
    }

    /// Returns the material to its default alignment *and* to the world-aligned paraxial
    /// axes, dropping a Valve face's free UV axes.
    ///
    /// `resetAllToParaxial` / "Reset UV alignment to world aligned": the same as
    /// [`Self::reset_uv`] for a Standard face, but for a Valve face it also forces the
    /// paraxial system, which is the one whose axes follow the world.
    pub fn reset_uv_to_paraxial(&mut self) {
        self.primitive_uv = None;
        self.uv_attributes = UvAttributes::default();
        self.uv_coord_system = UvCoordSystem::Paraxial;
    }

    /// Scales and shifts the material so that exactly one copy of it covers the face.
    ///
    /// `material_size` is the material's dimensions in texels. The face is given as the
    /// polygon it covers, which the brush's geometry provides.
    pub fn fit_uv_to_face(&mut self, polygon: &[Vec3d], material_size: Vec2f) {
        if polygon.is_empty() || material_size.x() <= 0.0 || material_size.y() <= 0.0 {
            return;
        }

        // Start from a clean mapping, so the fit does not compound whatever was there.
        self.reset_uv();

        // How far the face reaches across its own UV axes.
        let coords: Vec<Vec2f> = polygon.iter().map(|p| self.uv_coords(*p)).collect();
        let min = coords
            .iter()
            .fold(coords[0], |acc, c| geometry_math::vec::min(acc, *c));
        let max = coords
            .iter()
            .fold(coords[0], |acc, c| geometry_math::vec::max(acc, *c));
        let extent = max - min;

        // Scale so the face's extent becomes the material's, then shift its corner to the
        // material's origin.
        let scale = Vec2f::new(
            if extent.x() > 0.0 {
                extent.x() / material_size.x()
            } else {
                1.0
            },
            if extent.y() > 0.0 {
                extent.y() / material_size.y()
            } else {
                1.0
            },
        );
        self.uv_attributes.scale = scale;
        self.uv_attributes.offset = Vec2f::new(-min.x() / scale.x(), -min.y() / scale.y());
    }

    /// Returns three points on this face's plane, suitable for writing back out.
    ///
    /// A face built by the editor rather than read from a file has no points of its own,
    /// so this derives a set: the plane's anchor and two points along axes lying in the
    /// plane, wound to give back the same normal.
    pub fn derive_points(&self) -> [Vec3d; 3] {
        let normal = self.plane.normal;
        let anchor = self.plane.anchor();

        // Cross with whichever axis the normal is least aligned with, so the result
        // cannot be degenerate.
        let reference = match normal.find_abs_max_component(0) {
            0 => Vec3d::new(0.0, 0.0, 1.0),
            _ => Vec3d::new(1.0, 0.0, 0.0),
        };
        let u = vec::normalize(vec::cross(normal, reference));
        let v = vec::cross(normal, u);

        // Wound so that `plane::from_points` gives back this face's normal.
        [anchor, anchor + v, anchor + u]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uv::UvCoordSystem;

    /// The top face of a 32 unit cube at the origin, wound clockwise seen from above.
    fn top_face() -> (BrushFace, Vec<Vec3d>) {
        let polygon = vec![
            Vec3d::new(0.0, 0.0, 32.0),
            Vec3d::new(0.0, 32.0, 32.0),
            Vec3d::new(32.0, 32.0, 32.0),
            Vec3d::new(32.0, 0.0, 32.0),
        ];
        let face = BrushFace::from_points(
            [polygon[0], polygon[1], polygon[2]],
            "stone",
            UvCoordSystem::Paraxial,
            UvAttributes::default(),
        )
        .expect("the three points define a plane");
        (face, polygon)
    }

    fn three_stops() -> GradientStops {
        GradientStops::new(&[
            ColorStop {
                position: 0.0,
                color: Rgb::new(255, 0, 0),
            },
            ColorStop {
                position: 0.5,
                color: Rgb::new(0, 255, 0),
            },
            ColorStop {
                position: 1.0,
                color: Rgb::new(0, 0, 255),
            },
        ])
        .unwrap()
    }

    #[test]
    fn multicolor_gradient_samples_stops_and_rejects_invalid_positions() {
        let stops = three_stops();
        assert_eq!(
            GradientStops::new(&[
                stops.as_slice()[2],
                stops.as_slice()[0],
                stops.as_slice()[1]
            ])
            .unwrap(),
            stops
        );
        assert_eq!(stops.at(0.5), [0.0, 1.0, 0.0]);
        assert_eq!(stops.at(0.25), [0.5, 0.5, 0.0]);
        assert_eq!(stops.at(-1.0), [1.0, 0.0, 0.0]);
        assert_eq!(stops.at(2.0), [0.0, 0.0, 1.0]);
        assert!(GradientStops::new(&[]).is_none());
        assert!(GradientStops::new(
            &[ColorStop {
                position: f32::NAN,
                color: Rgb::new(0, 0, 0)
            }; 2]
        )
        .is_none());
        assert!(GradientStops::new(
            &[ColorStop {
                position: 0.0,
                color: Rgb::new(0, 0, 0)
            }; 9]
        )
        .is_none());
    }

    #[test]
    fn radial_gradient_has_a_coloured_centre_and_preserves_triangle_area() {
        let (face, polygon) = top_face();
        let paint = FaceColor::MultiGradient {
            stops: three_stops(),
            radial: true,
            angle: 0.0,
            alpha: 123,
            ends: None,
        };
        let sampler = PaintOnFace::new(&face, &polygon, paint);
        let centre =
            polygon.iter().copied().fold(Vec3d::zero(), |a, b| a + b) / polygon.len() as f64;
        assert_eq!(sampler.at(&face, centre), [1.0, 0.0, 0.0]);
        assert_eq!(sampler.at(&face, polygon[0]), [0.0, 0.0, 1.0]);
        let mesh = paint_triangles(&face, &polygon, Some(paint));
        assert!(mesh
            .iter()
            .flatten()
            .any(|p| sampler.at(&face, *p) == [1.0, 0.0, 0.0]));
        let area = |triangles: Vec<[Vec3d; 3]>| {
            triangles
                .iter()
                .map(|t| vec::length(vec::cross(t[1] - t[0], t[2] - t[0])) / 2.0)
                .sum::<f64>()
        };
        assert!((area(mesh) - area(paint_triangles(&face, &polygon, None))).abs() < 1e-6);
        assert_eq!(paint.with_alpha(44).alpha(), 44);
    }

    #[test]
    fn a_gradient_placed_on_the_face_runs_between_its_ends() {
        let (face, polygon) = top_face();
        let stops = GradientStops::new(&[
            ColorStop {
                position: 0.0,
                color: Rgb::new(255, 0, 0),
            },
            ColorStop {
                position: 1.0,
                color: Rgb::new(0, 0, 255),
            },
        ])
        .unwrap();
        let ends = GradientEnds {
            start: [0.25, 0.5],
            end: [0.75, 0.5],
        };
        for radial in [false, true] {
            let paint = FaceColor::MultiGradient {
                stops,
                radial,
                angle: 0.0,
                alpha: 255,
                ends: Some(ends),
            };
            let sampler = PaintOnFace::new(&face, &polygon, paint);
            let start = sampler.from_bounds(ends.start);
            let end = sampler.from_bounds(ends.end);
            assert_eq!(sampler.at_uv(start), [1.0, 0.0, 0.0]);
            assert_eq!(sampler.at_uv(end), [0.0, 0.0, 1.0]);
            assert!((sampler.position_at_uv(start + (end - start) * 0.5) - 0.5).abs() < 1e-5);
            assert_eq!(sampler.ends(), Some(ends));
            let mesh = paint_triangles(&face, &polygon, Some(paint));
            let area = |triangles: Vec<[Vec3d; 3]>| {
                triangles
                    .iter()
                    .map(|t| vec::length(vec::cross(t[1] - t[0], t[2] - t[0])) / 2.0)
                    .sum::<f64>()
            };
            let (painted, whole) = (area(mesh), area(paint_triangles(&face, &polygon, None)));
            assert!(
                (painted - whole).abs() < 1e-3,
                "radial {radial}: {painted} of {whole}"
            );
        }
    }

    #[test]
    fn a_fitted_gradient_has_ends_that_paint_the_same() {
        let (face, polygon) = top_face();
        let fitted = FaceColor::MultiGradient {
            stops: three_stops(),
            radial: false,
            angle: 37.0,
            alpha: 255,
            ends: None,
        };
        let ends = PaintOnFace::new(&face, &polygon, fitted).ends().unwrap();
        let placed = FaceColor::MultiGradient {
            stops: three_stops(),
            radial: false,
            angle: 37.0,
            alpha: 255,
            ends: Some(ends),
        };
        let (a, b) = (
            PaintOnFace::new(&face, &polygon, fitted),
            PaintOnFace::new(&face, &polygon, placed),
        );
        for corner in &polygon {
            let (x, y) = (a.at(&face, *corner), b.at(&face, *corner));
            assert!(
                x.iter().zip(y).all(|(x, y)| (x - y).abs() < 1e-4),
                "{x:?} {y:?}"
            );
        }
    }

    #[test]
    fn linear_gradient_mesh_keeps_narrow_interior_stops() {
        let (face, polygon) = top_face();
        let stops = GradientStops::new(&[
            ColorStop {
                position: 0.0,
                color: Rgb::new(0, 0, 0),
            },
            ColorStop {
                position: 0.501,
                color: Rgb::new(255, 0, 0),
            },
            ColorStop {
                position: 0.502,
                color: Rgb::new(0, 255, 0),
            },
            ColorStop {
                position: 1.0,
                color: Rgb::new(0, 0, 255),
            },
        ])
        .unwrap();
        let paint = FaceColor::MultiGradient {
            stops,
            radial: false,
            angle: 37.0,
            alpha: 255,
            ends: None,
        };
        let sampler = PaintOnFace::new(&face, &polygon, paint);
        let mesh = paint_triangles(&face, &polygon, Some(paint));
        assert!(mesh.len() < 30);
        for expected in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            assert!(mesh.iter().flatten().any(|p| sampler
                .at(&face, *p)
                .iter()
                .zip(expected)
                .all(|(a, b)| (a - b).abs() < 0.001)));
        }
    }

    #[test]
    fn shearing_about_a_point_keeps_that_point_still() {
        let (mut face, _) = top_face();
        face.uv_coord_system = UvCoordSystem::Parallel {
            u_axis: Vec3d::new(1.0, 0.0, 0.0),
            v_axis: Vec3d::new(0.0, -1.0, 0.0),
        };
        face.uv_attributes.offset = Vec2f::new(5.0, -3.0);
        face.uv_attributes.scale = Vec2f::new(0.5, 2.0);
        let anchor = Vec3d::new(20.0, 12.0, 32.0);
        let origin = face.uv_coords(anchor);

        face.shear_uv_about(origin, Vec2f::new(0.4, 0.0));

        let after = face.uv_coords(anchor);
        assert!(
            (after.x() - origin.x()).abs() < 1e-3,
            "{after:?} vs {origin:?}"
        );
        assert!(
            (after.y() - origin.y()).abs() < 1e-3,
            "{after:?} vs {origin:?}"
        );
        // And the material really is skewed: a point along U now has a different V.
        let along_u = face.uv_coords(anchor + Vec3d::new(10.0, 0.0, 0.0));
        assert!(
            (along_u.y() - origin.y()).abs() > 1e-3,
            "nothing was sheared"
        );
    }

    #[test]
    fn a_gradient_runs_from_one_side_of_the_face_to_the_other() {
        let (face, polygon) = top_face();
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        let gradient = GradientOnFace::new(&face, &polygon, black, white, 0.0);

        // Whichever corner comes first along the gradient's direction is the start
        // colour and whichever comes last is the end, with the middle halfway between.
        let colours: Vec<f32> = polygon
            .iter()
            .map(|corner| gradient.at(&face, *corner)[0])
            .collect();
        assert!(
            colours.iter().any(|c| *c < 1e-6),
            "a corner is the start colour"
        );
        assert!(
            colours.iter().any(|c| *c > 1.0 - 1e-6),
            "a corner is the end colour"
        );

        let middle = gradient.at(&face, Vec3d::new(16.0, 16.0, 32.0))[0];
        assert!(
            (middle - 0.5).abs() < 1e-6,
            "the middle is halfway: {middle}"
        );
    }

    #[test]
    fn turning_a_gradient_by_ninety_degrees_runs_it_the_other_way() {
        let (face, polygon) = top_face();
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        let across = GradientOnFace::new(&face, &polygon, black, white, 0.0);
        let down = GradientOnFace::new(&face, &polygon, black, white, 90.0);

        // A corner that is at one end of the first gradient is halfway along neither: the
        // two run along different axes, so they disagree somewhere.
        let disagrees = polygon
            .iter()
            .any(|corner| (across.at(&face, *corner)[0] - down.at(&face, *corner)[0]).abs() > 0.5);
        assert!(disagrees, "the two gradients run the same way");
    }

    #[test]
    fn a_gradient_on_a_face_with_no_extent_gives_the_starting_colour() {
        let (face, _) = top_face();
        let point = Vec3d::new(0.0, 0.0, 32.0);
        let gradient = GradientOnFace::new(
            &face,
            &[point],
            Rgb::new(10, 20, 30),
            Rgb::new(200, 200, 200),
            0.0,
        );
        assert_eq!(gradient.at(&face, point), Rgb::new(10, 20, 30).to_f32());
    }
}

#[cfg(test)]
mod size_tests {
    use super::*;

    /// A map holds a hundred thousand of these, so the size of one is worth pinning.
    ///
    /// PBR parameters add 8 bytes including alignment, bringing the face to 248 bytes.
    /// This costs 0.8 MB per 100,000 faces without any per-face heap allocation.
    /// Keep the bound exact so future additions must account for their memory cost.
    #[test]
    fn a_face_stays_small() {
        let size = std::mem::size_of::<BrushFace>();
        assert!(size <= 248, "BrushFace has grown to {size} bytes");
    }
}
