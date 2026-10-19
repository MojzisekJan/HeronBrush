//! Reading the models a point entity is drawn as.
//!
//! Reads Quake `.mdl` entity models referenced by FGD model specifications.
//!
//! Loads only the first frame for static editor previews.
//!
//! The layout is the one `qfiles.h` describes: a header, the skins as palette indices, a
//! texture coordinate per vertex, the triangles as indices, and then the frames, whose
//! vertices are bytes scaled and shifted by the header.

use geometry_math::bbox::Bbox3d;
use geometry_math::vec::{Vec2f, Vec3d};

/// A model, ready to be drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityModel {
    /// The triangles, in the model's own space, with the first frame's vertices.
    pub triangles: Vec<ModelTriangle>,
    /// The first skin, as RGBA.
    pub skin: Skin,
    /// Additional material skins; triangle material zero uses `skin`.
    pub extra_skins: Vec<Skin>,
    /// glTF PBR materials in skin order; empty for legacy formats.
    pub pbr: Vec<crate::model_pbr::ModelPbr>,
    /// The box the model fills, which is what a flat view draws and what a click hits.
    pub bounds: Bbox3d,
}

impl EntityModel {
    /// Optional PBR data for a skin.
    pub fn pbr_at(&self, material: usize) -> Option<&crate::model_pbr::ModelPbr> { self.pbr.get(material) }
    /// Skins in triangle material order.
    pub fn skins(&self) -> impl Iterator<Item = &Skin> {
        std::iter::once(&self.skin).chain(self.extra_skins.iter())
    }
    /// Skin selected by a validated triangle material index.
    pub fn skin_at(&self, material: usize) -> &Skin {
        if material == 0 {
            &self.skin
        } else {
            &self.extra_skins[material - 1]
        }
    }
}

/// One triangle of a model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelTriangle {
    /// Its three corners, in the model's own space.
    pub positions: [Vec3d; 3],
    /// Where each corner sits on the skin, in texels.
    pub uv: [Vec2f; 3],
    /// Zero uses the primary skin, higher indices use `extra_skins`.
    pub material: usize,
    /// Supplied unit normals in model space, or geometric face shading.
    pub normals: Option<[Vec3d; 3]>,
}

/// A model's texture.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Skin {
    /// Across.
    pub width: u32,
    /// Down.
    pub height: u32,
    /// Four bytes a pixel, top row first, as [`crate::material::Material`] holds them.
    pub pixels: Vec<u8>,
}

/// Why a model could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// The file does not begin with the magic a `.mdl` begins with.
    NotAModel,
    /// A version this does not know how to read.
    Version(i32),
    /// The file ends before what its header says is in it.
    Truncated,
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::NotAModel => write!(out, "not a Quake model"),
            ModelError::Version(version) => write!(out, "model version {version} is not read here"),
            ModelError::Truncated => write!(out, "the model ends early"),
        }
    }
}

impl std::error::Error for ModelError {}

/// Reads a `.mdl`, colouring its skin with the given palette.
///
/// The palette is 256 colours of three bytes, as `palette.lmp` holds them; without one the
/// skin comes out grey, since a model with no colours is still worth its shape.
pub fn read(bytes: &[u8], palette: Option<&[u8]>) -> Result<EntityModel, ModelError> {
    let mut at = Reader::new(bytes);
    if at.take(4)? != b"IDPO" {
        return Err(ModelError::NotAModel);
    }
    let version = at.int()?;
    if version != 6 {
        return Err(ModelError::Version(version));
    }

    let scale = at.vector()?;
    let translate = at.vector()?;
    let _bounding_radius = at.float()?;
    let _eye_position = at.vector()?;
    let skins = at.int()?.max(0) as usize;
    let skin_width = at.int()?.max(0) as usize;
    let skin_height = at.int()?.max(0) as usize;
    let vertices = at.int()?.max(0) as usize;
    let triangles = at.int()?.max(0) as usize;
    let frames = at.int()?.max(0) as usize;
    let _sync_type = at.int()?;
    let _flags = at.int()?;
    let _size = at.float()?;

    // The skins. Only the first is kept -- a model in an editor wears one -- but every one
    // has to be stepped over to reach the texture coordinates.
    let texels = skin_width * skin_height;
    let mut first_skin: Option<&[u8]> = None;
    for _ in 0..skins {
        let group = at.int()?;
        if group == 0 {
            let pixels = at.take(texels)?;
            first_skin.get_or_insert(pixels);
        } else {
            // A group of skins is an animation; its first is as good as any.
            let count = at.int()?.max(0) as usize;
            at.take(count * 4)?;
            for index in 0..count {
                let pixels = at.take(texels)?;
                if index == 0 {
                    first_skin.get_or_insert(pixels);
                }
            }
        }
    }

    // A texture coordinate per vertex, and a flag saying whether the vertex sits on the
    // seam that the front and back halves of the skin meet at.
    let mut coordinates = Vec::with_capacity(vertices);
    for _ in 0..vertices {
        let on_seam = at.int()? != 0;
        let s = at.int()?;
        let t = at.int()?;
        coordinates.push((on_seam, s, t));
    }

    let mut faces = Vec::with_capacity(triangles);
    for _ in 0..triangles {
        let faces_front = at.int()? != 0;
        let corners = [at.int()?, at.int()?, at.int()?];
        faces.push((faces_front, corners));
    }

    // The first frame, and only it.
    if frames == 0 {
        return Err(ModelError::Truncated);
    }
    let kind = at.int()?;
    if kind != 0 {
        // A group of frames: a count, its own bounds, the times, then the frames.
        let count = at.int()?.max(0) as usize;
        at.take(8)?;
        at.take(count * 4)?;
    }
    // Each frame begins with its own bounds and name.
    at.take(8)?;
    at.take(16)?;
    let mut positions = Vec::with_capacity(vertices);
    for _ in 0..vertices {
        let packed = at.take(4)?;
        positions.push(Vec3d::new(
            f64::from(packed[0]) * scale.x() + translate.x(),
            f64::from(packed[1]) * scale.y() + translate.y(),
            f64::from(packed[2]) * scale.z() + translate.z(),
        ));
    }

    let mut out = Vec::with_capacity(faces.len());
    let mut bounds: Option<Bbox3d> = None;
    for (faces_front, corners) in faces {
        let mut points = [Vec3d::zero(); 3];
        let mut uv = [Vec2f::new(0.0, 0.0); 3];
        for (at, corner) in corners.iter().enumerate() {
            let index = usize::try_from(*corner).map_err(|_| ModelError::Truncated)?;
            let position = *positions.get(index).ok_or(ModelError::Truncated)?;
            let (on_seam, s, t) = *coordinates.get(index).ok_or(ModelError::Truncated)?;
            // A vertex on the seam belongs to the right half of the skin when the
            // triangle faces away from the viewer, which is how one skin covers both
            // sides of a model.
            let s = if on_seam && !faces_front {
                s + skin_width as i32 / 2
            } else {
                s
            };
            points[at] = position;
            uv[at] = Vec2f::new(s as f32, t as f32);
            bounds = Some(match bounds {
                Some(box_so_far) => box_so_far.merged(&Bbox3d::new(position, position)),
                None => Bbox3d::new(position, position),
            });
        }
        out.push(ModelTriangle {
            material: 0,
            normals: None,
            positions: points,
            uv,
        });
    }

    Ok(EntityModel {
        triangles: out,
        extra_skins: vec![],
        pbr: vec![],
        skin: colour(first_skin, skin_width, skin_height, palette),
        bounds: bounds.unwrap_or_else(|| Bbox3d::new(Vec3d::zero(), Vec3d::zero())),
    })
}

/// Turns a skin's palette indices into the RGBA a material holds.
fn colour(pixels: Option<&[u8]>, width: usize, height: usize, palette: Option<&[u8]>) -> Skin {
    let mut rgba = Vec::with_capacity(width * height * 4);
    match (pixels, palette) {
        (Some(pixels), Some(palette)) if palette.len() >= 768 => {
            for index in pixels {
                let at = usize::from(*index) * 3;
                rgba.extend_from_slice(&[palette[at], palette[at + 1], palette[at + 2], 255]);
            }
        }
        (Some(pixels), _) => {
            // No palette to colour it with: the index itself as a grey, so the model is at
            // least shaded rather than flat.
            for index in pixels {
                rgba.extend_from_slice(&[*index, *index, *index, 255]);
            }
        }
        (None, _) => rgba.extend(std::iter::repeat_n(255, width * height * 4)),
    }
    Skin {
        width: width as u32,
        height: height as u32,
        pixels: rgba,
    }
}

/// A cursor over the bytes, which is all the reading here needs.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], ModelError> {
        let end = self.at.checked_add(count).ok_or(ModelError::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(ModelError::Truncated)?;
        self.at = end;
        Ok(slice)
    }

    fn int(&mut self) -> Result<i32, ModelError> {
        let bytes = self.take(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn float(&mut self) -> Result<f64, ModelError> {
        let bytes = self.take(4)?;
        Ok(f64::from(f32::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ])))
    }

    fn vector(&mut self) -> Result<Vec3d, ModelError> {
        Ok(Vec3d::new(self.float()?, self.float()?, self.float()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A model of one triangle, written the way a `.mdl` is.
    fn one_triangle() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"IDPO");
        out.extend_from_slice(&6i32.to_le_bytes());
        for value in [1.0f32, 1.0, 1.0] {
            out.extend_from_slice(&value.to_le_bytes()); // scale
        }
        for value in [0.0f32, 0.0, 0.0] {
            out.extend_from_slice(&value.to_le_bytes()); // translate
        }
        out.extend_from_slice(&1.0f32.to_le_bytes()); // bounding radius
        for value in [0.0f32, 0.0, 0.0] {
            out.extend_from_slice(&value.to_le_bytes()); // eye position
        }
        out.extend_from_slice(&1i32.to_le_bytes()); // skins
        out.extend_from_slice(&2i32.to_le_bytes()); // skin width
        out.extend_from_slice(&2i32.to_le_bytes()); // skin height
        out.extend_from_slice(&3i32.to_le_bytes()); // vertices
        out.extend_from_slice(&1i32.to_le_bytes()); // triangles
        out.extend_from_slice(&1i32.to_le_bytes()); // frames
        out.extend_from_slice(&0i32.to_le_bytes()); // sync type
        out.extend_from_slice(&0i32.to_le_bytes()); // flags
        out.extend_from_slice(&1.0f32.to_le_bytes()); // size

        out.extend_from_slice(&0i32.to_le_bytes()); // one skin, not a group
        out.extend_from_slice(&[0, 1, 2, 3]); // four palette indices

        for (on_seam, s, t) in [(0i32, 0i32, 0i32), (0, 1, 0), (1, 0, 1)] {
            out.extend_from_slice(&on_seam.to_le_bytes());
            out.extend_from_slice(&s.to_le_bytes());
            out.extend_from_slice(&t.to_le_bytes());
        }

        out.extend_from_slice(&0i32.to_le_bytes()); // faces away from the viewer
        for corner in [0i32, 1, 2] {
            out.extend_from_slice(&corner.to_le_bytes());
        }

        out.extend_from_slice(&0i32.to_le_bytes()); // a single frame
        out.extend_from_slice(&[0, 0, 0, 0]); // its bounds
        out.extend_from_slice(&[16, 16, 16, 0]);
        out.extend_from_slice(&[b'f'; 16]); // its name
        for vertex in [[0u8, 0, 0, 0], [16, 0, 0, 0], [0, 16, 0, 0]] {
            out.extend_from_slice(&vertex);
        }
        out
    }

    #[test]
    fn a_model_reads_as_its_first_frame() {
        let model = read(&one_triangle(), None).expect("it reads");
        assert_eq!(model.triangles.len(), 1);
        assert_eq!(model.triangles[0].positions[1], Vec3d::new(16.0, 0.0, 0.0));
        assert_eq!(model.bounds.max, Vec3d::new(16.0, 16.0, 0.0));
    }

    #[test]
    fn a_vertex_on_the_seam_takes_the_far_half_of_the_skin() {
        // The triangle faces away from the viewer, so its seam vertex belongs to the
        // right-hand half: half of a two-texel skin is one.
        let model = read(&one_triangle(), None).expect("it reads");
        assert_eq!(model.triangles[0].uv[2], Vec2f::new(1.0, 1.0));
        assert_eq!(model.triangles[0].uv[0], Vec2f::new(0.0, 0.0));
    }

    #[test]
    fn the_skin_is_coloured_by_the_palette() {
        let mut palette = vec![0u8; 768];
        palette[3] = 200; // colour 1 is red
        let model = read(&one_triangle(), Some(&palette)).expect("it reads");
        assert_eq!(model.skin.width, 2);
        assert_eq!(model.skin.pixels.len(), 4 * 4);
        assert_eq!(&model.skin.pixels[4..8], &[200, 0, 0, 255]);
    }

    #[test]
    fn something_that_is_not_a_model_is_an_error() {
        assert_eq!(read(b"nope", None), Err(ModelError::NotAModel));
        assert_eq!(
            read(&one_triangle()[..20], None),
            Err(ModelError::Truncated)
        );
    }
}
