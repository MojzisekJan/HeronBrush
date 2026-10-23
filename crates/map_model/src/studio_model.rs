//! Half-Life studio models (`IDST`, version 10), the `.mdl` every monster and item in
//! a GoldSrc game is drawn with.
//!
//! Layout reference: Valve's `studio.h` in the Half-Life SDK (`engine/studio.h` in
//! FWGS's hlsdk-portable). Unlike a Quake model, the vertices are stored in the space of
//! the bone each one hangs on, so the first frame of the first sequence is played to
//! place the bones before a vertex can be put anywhere. Only the first model of each
//! body part and the first skin family are read: the editor draws one still pose.
//!
//! The textures are 8-bit with a palette after each one. A model whose textures would
//! be too big for one file keeps them in a second, `<name>t.mdl`; the caller hands that
//! over when asked.

use crate::entity_model::{EntityModel, ModelTriangle, Skin};
use geometry_math::bbox::Bbox3d;
use geometry_math::vec::{Vec2f, Vec3d};

const HEADER_SIZE: usize = 244;
const BONE_SIZE: usize = 112;
const BODY_PART_SIZE: usize = 76;
const MESH_SIZE: usize = 20;
const TEXTURE_SIZE: usize = 80;
/// `STUDIO_NF_MASKED`: the last palette entry is a hole.
const MASKED: i32 = 0x40;
/// More than any shipped model has, and few enough that a corrupt count cannot ask for
/// gigabytes.
const LIMIT: usize = 1 << 16;

/// Whether the bytes are a Half-Life studio model rather than a Quake one.
pub fn is_studio_model(bytes: &[u8]) -> bool {
    bytes.get(..4) == Some(b"IDST")
}

/// Reads a studio model. `texture_file` is asked for the `<name>t.mdl` beside it when
/// the model keeps no textures of its own; without one the model is drawn grey.
pub fn read(
    bytes: &[u8],
    texture_file: &mut dyn FnMut() -> Option<Vec<u8>>,
) -> Result<EntityModel, String> {
    let file = File { bytes };
    if !is_studio_model(bytes) {
        return Err("not a Half-Life studio model".into());
    }
    let version = file.int(4)?;
    if version != 10 {
        return Err(format!("studio model version {version} is not read here"));
    }
    if bytes.len() < HEADER_SIZE {
        return Err(truncated());
    }

    let bones = place_bones(&file)?;

    // The textures: in this file, or in the companion one.
    let companion;
    let textures_from = if file.count(180)? == 0 {
        companion = texture_file();
        companion
            .as_deref()
            .filter(|bytes| is_studio_model(bytes))
            .map(|bytes| File { bytes })
    } else {
        Some(File { bytes })
    };
    let skins = match &textures_from {
        Some(textures) => read_textures(textures)?,
        None => Vec::new(),
    };
    // Which texture a mesh's skin reference names, in the first skin family. The table
    // lives with the textures.
    let skin_table: Vec<usize> = match &textures_from {
        Some(textures) => {
            let references = textures.count(192)?;
            let at = textures.offset(200)?;
            (0..references)
                .map(|i| textures.short(at + i * 2).map(|v| v.max(0) as usize))
                .collect::<Result<_, _>>()?
        }
        None => Vec::new(),
    };

    let mut triangles = Vec::new();
    let body_parts = file.count(204)?;
    let body_part_at = file.offset(208)?;
    for part in 0..body_parts {
        let at = body_part_at + part * BODY_PART_SIZE;
        if file.count(at + 64)? == 0 {
            continue;
        }
        // The first model of the part: what a body value of 0 shows.
        let model = file.offset(at + 72)?;
        read_model(&file, model, &bones, &skin_table, &skins, &mut triangles)?;
    }

    let mut bounds: Option<Bbox3d> = None;
    for triangle in &triangles {
        for p in triangle.positions {
            let point = Bbox3d::new(p, p);
            bounds = Some(bounds.map_or(point, |b| b.merged(&point)));
        }
    }
    let bounds = bounds.ok_or("the studio model has no triangles")?;

    let grey = || Skin {
        width: 1,
        height: 1,
        pixels: vec![160, 160, 160, 255],
    };
    let mut skins = skins.into_iter();
    let skin = skins.next().unwrap_or_else(grey);
    let extra_skins: Vec<Skin> = skins.collect();
    // A triangle whose texture is missing falls back on the first skin.
    for triangle in &mut triangles {
        if triangle.material > extra_skins.len() {
            triangle.material = 0;
        }
    }
    Ok(EntityModel {
        triangles,
        skin,
        extra_skins,
        pbr: vec![],
        bounds,
    })
}

/// A 3x4 matrix: rotation in the first three columns, translation in the last.
type Transform = [[f64; 4]; 3];

/// Plays frame 0 of sequence 0 and returns every bone's transform into model space.
fn place_bones(file: &File) -> Result<Vec<Transform>, String> {
    let count = file.count(140)?;
    let bone_at = file.offset(144)?;
    // Sequence 0's animation, when it is in this file. A sequence in another group is
    // in a `<name>01.mdl`, which is not read; the bones then keep their rest values.
    let animation = if file.count(164)? > 0 {
        let sequence = file.offset(168)?;
        if file.int(sequence + 156)? == 0 {
            Some(file.offset(sequence + 124)?)
        } else {
            None
        }
    } else {
        None
    };

    let mut placed: Vec<Transform> = Vec::with_capacity(count);
    for bone in 0..count {
        let at = bone_at + bone * BONE_SIZE;
        let mut values = [0.0f64; 6];
        for (channel, value) in values.iter_mut().enumerate() {
            let rest = f64::from(file.float(at + 64 + channel * 4)?);
            let scale = f64::from(file.float(at + 88 + channel * 4)?);
            let stored = match animation {
                Some(animation) => frame_zero_value(file, animation + bone * 12, channel)?,
                None => 0.0,
            };
            *value = rest + stored * scale;
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err("a studio model bone is not a number".into());
        }
        let local = transform(&values);
        let parent = file.int(at + 32)?;
        let world = if parent >= 0 && (parent as usize) < bone {
            concat(&placed[parent as usize], &local)
        } else {
            local
        };
        placed.push(world);
    }
    Ok(placed)
}

/// The value one channel of a bone has in frame 0, before the bone's scale: what
/// `StudioCalcBoneAngles` and `StudioCalcBonePosition` read for frame 0.
fn frame_zero_value(file: &File, animation: usize, channel: usize) -> Result<f64, String> {
    let offset = file.ushort(animation + channel * 2)? as usize;
    if offset == 0 {
        return Ok(0.0);
    }
    let values = animation + offset;
    // The first entry is a header: how many values are stored, and how many frames
    // they cover. Frame 0 is the first stored value.
    let valid = file.byte(values)?;
    let total = file.byte(values + 1)?;
    if total == 0 {
        return Err("a studio model animation covers no frames".into());
    }
    let index = if valid > 0 { 1 } else { 0 };
    Ok(f64::from(file.short(values + index * 2)?))
}

/// A bone's local transform from its position and its angles: roll, pitch and yaw in
/// radians, turned into a quaternion the way `AngleQuaternion` does.
fn transform(values: &[f64; 6]) -> Transform {
    let (sy, cy) = (values[5] * 0.5).sin_cos();
    let (sp, cp) = (values[4] * 0.5).sin_cos();
    let (sr, cr) = (values[3] * 0.5).sin_cos();
    let x = sr * cp * cy - cr * sp * sy;
    let y = cr * sp * cy + sr * cp * sy;
    let z = cr * cp * sy - sr * sp * cy;
    let w = cr * cp * cy + sr * sp * sy;
    [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - w * z),
            2.0 * (x * z + w * y),
            values[0],
        ],
        [
            2.0 * (x * y + w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - w * x),
            values[1],
        ],
        [
            2.0 * (x * z - w * y),
            2.0 * (y * z + w * x),
            1.0 - 2.0 * (x * x + y * y),
            values[2],
        ],
    ]
}

/// `parent * child`, as `R_ConcatTransforms` does it.
fn concat(parent: &Transform, child: &Transform) -> Transform {
    let mut out = [[0.0; 4]; 3];
    for (row, out_row) in out.iter_mut().enumerate() {
        for column in 0..4 {
            let mut sum = (0..3)
                .map(|k| parent[row][k] * child[k][column])
                .sum::<f64>();
            if column == 3 {
                sum += parent[row][3];
            }
            out_row[column] = sum;
        }
    }
    out
}

fn apply(transform: &Transform, point: Vec3d) -> Vec3d {
    let row = |r: &[f64; 4]| r[0] * point.x() + r[1] * point.y() + r[2] * point.z() + r[3];
    Vec3d::new(row(&transform[0]), row(&transform[1]), row(&transform[2]))
}

fn read_model(
    file: &File,
    at: usize,
    bones: &[Transform],
    skin_table: &[usize],
    skins: &[Skin],
    triangles: &mut Vec<ModelTriangle>,
) -> Result<(), String> {
    let meshes = file.count(at + 72)?;
    let mesh_at = file.offset(at + 76)?;
    let vertex_count = file.count(at + 80)?;
    let bone_of_at = file.offset(at + 84)?;
    let vertex_at = file.offset(at + 88)?;

    let mut vertices = Vec::with_capacity(vertex_count);
    for vertex in 0..vertex_count {
        let bone = file.byte(bone_of_at + vertex)? as usize;
        let transform = bones
            .get(bone)
            .ok_or("a studio model vertex hangs on a bone that is not there")?;
        let at = vertex_at + vertex * 12;
        let local = Vec3d::new(
            f64::from(file.float(at)?),
            f64::from(file.float(at + 4)?),
            f64::from(file.float(at + 8)?),
        );
        vertices.push(apply(transform, local));
    }

    for mesh in 0..meshes {
        let at = mesh_at + mesh * MESH_SIZE;
        let commands = file.offset(at + 4)?;
        let reference = file.count(at + 8)?;
        let texture = skin_table.get(reference).copied().unwrap_or(reference);
        let size = skins
            .get(texture)
            .map(|skin| (skin.width as f32, skin.height as f32));
        let material = if size.is_some() { texture } else { 0 };
        read_commands(file, commands, &vertices, material, triangles)?;
    }
    Ok(())
}

/// Turns a mesh's triangle commands into triangles: each command is a count, negative
/// for a fan and positive for a strip, then that many corners of four shorts (vertex,
/// normal, s, t). A count of zero ends the list.
fn read_commands(
    file: &File,
    mut at: usize,
    vertices: &[Vec3d],
    material: usize,
    triangles: &mut Vec<ModelTriangle>,
) -> Result<(), String> {
    for _ in 0..LIMIT {
        let count = file.short(at)?;
        at += 2;
        if count == 0 {
            return Ok(());
        }
        let fan = count < 0;
        let corners = usize::from(count.unsigned_abs());
        let mut list = Vec::with_capacity(corners);
        for _ in 0..corners {
            let vertex = file.short(at)?;
            let s = file.short(at + 4)?;
            let t = file.short(at + 6)?;
            at += 8;
            let position = usize::try_from(vertex)
                .ok()
                .and_then(|i| vertices.get(i))
                .ok_or("a studio model triangle names a vertex that is not there")?;
            list.push((*position, Vec2f::new(f32::from(s), f32::from(t))));
        }
        for i in 2..list.len() {
            let [a, b, c] = if fan {
                [0, i - 1, i]
            } else if i % 2 == 0 {
                [i - 2, i - 1, i]
            } else {
                [i - 1, i - 2, i]
            };
            triangles.push(ModelTriangle {
                positions: [list[a].0, list[b].0, list[c].0],
                uv: [list[a].1, list[b].1, list[c].1],
                material,
                normals: None,
            });
        }
    }
    Err("a studio model mesh never ends its triangle commands".into())
}

/// Every texture in the file, as RGBA, each coloured by the palette that follows it.
fn read_textures(file: &File) -> Result<Vec<Skin>, String> {
    let count = file.count(180)?;
    let at = file.offset(184)?;
    let mut skins = Vec::with_capacity(count);
    for texture in 0..count {
        let header = at + texture * TEXTURE_SIZE;
        let flags = file.int(header + 64)?;
        let width = file.count(header + 68)?;
        let height = file.count(header + 72)?;
        let pixels_at = file.offset(header + 76)?;
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err("a studio model texture has an impossible size".into());
        }
        let indices = file.slice(pixels_at, width * height)?;
        let palette = file.slice(pixels_at + width * height, 768)?;
        let masked = flags & MASKED != 0;
        let mut pixels = Vec::with_capacity(width * height * 4);
        for index in indices {
            let i = usize::from(*index) * 3;
            let alpha = if masked && *index == 255 { 0 } else { 255 };
            pixels.extend_from_slice(&[palette[i], palette[i + 1], palette[i + 2], alpha]);
        }
        skins.push(Skin {
            width: width as u32,
            height: height as u32,
            pixels,
        });
    }
    Ok(skins)
}

fn truncated() -> String {
    "the studio model ends early".into()
}

/// The file, read with bounds checks: every number in a studio model is an offset or a
/// count somebody could have got wrong.
struct File<'a> {
    bytes: &'a [u8],
}

impl File<'_> {
    fn slice(&self, at: usize, length: usize) -> Result<&[u8], String> {
        let end = at.checked_add(length).ok_or_else(truncated)?;
        self.bytes.get(at..end).ok_or_else(truncated)
    }

    fn array<const N: usize>(&self, at: usize) -> Result<[u8; N], String> {
        self.slice(at, N)?.try_into().map_err(|_| truncated())
    }

    fn int(&self, at: usize) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.array(at)?))
    }

    fn float(&self, at: usize) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.array(at)?))
    }

    fn short(&self, at: usize) -> Result<i16, String> {
        Ok(i16::from_le_bytes(self.array(at)?))
    }

    fn ushort(&self, at: usize) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.array(at)?))
    }

    fn byte(&self, at: usize) -> Result<u8, String> {
        self.bytes.get(at).copied().ok_or_else(truncated)
    }

    /// A count from the header or a record, refused when negative or past [`LIMIT`].
    fn count(&self, at: usize) -> Result<usize, String> {
        usize::try_from(self.int(at)?)
            .ok()
            .filter(|count| *count <= LIMIT)
            .ok_or_else(|| "a studio model count is out of range".into())
    }

    /// An offset into the file, refused when negative or past its end.
    fn offset(&self, at: usize) -> Result<usize, String> {
        usize::try_from(self.int(at)?)
            .ok()
            .filter(|offset| *offset <= self.bytes.len())
            .ok_or_else(truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEQUENCE_SIZE: usize = 176;
    const MODEL_SIZE: usize = 112;

    /// Builds a studio model in memory: two bones, the second turned a quarter about Z
    /// and moved up by 10 from its parent, one mesh of two strip triangles and one fan
    /// triangle on the second bone, and one 2x2 texture whose last pixel is a hole.
    fn model(with_textures: bool) -> Vec<u8> {
        let mut b = vec![0u8; HEADER_SIZE];
        b[..4].copy_from_slice(b"IDST");
        put_int(&mut b, 4, 10);

        // Bones.
        let bone_at = b.len();
        put_int(&mut b, 140, 2);
        put_int(&mut b, 144, bone_at as i32);
        b.resize(bone_at + 2 * BONE_SIZE, 0);
        put_int(&mut b, bone_at + 32, -1);
        for channel in 0..6 {
            put_float(&mut b, bone_at + 88 + channel * 4, 1.0);
        }
        let second = bone_at + BONE_SIZE;
        put_int(&mut b, second + 32, 0);
        // Rest position 0 0 10; the yaw comes from the animation, scaled to a quarter turn.
        put_float(&mut b, second + 64 + 8, 10.0);
        for channel in 0..6 {
            put_float(&mut b, second + 88 + channel * 4, 1.0);
        }
        put_float(
            &mut b,
            second + 88 + 5 * 4,
            std::f32::consts::FRAC_PI_2 / 157.0,
        );

        // One sequence in group 0 whose animation turns bone 1 by 157 steps of that scale.
        let sequence_at = b.len();
        put_int(&mut b, 164, 1);
        put_int(&mut b, 168, sequence_at as i32);
        b.resize(sequence_at + SEQUENCE_SIZE, 0);
        let animation_at = b.len();
        put_int(&mut b, sequence_at + 124, animation_at as i32);
        b.resize(animation_at + 24, 0);
        // Bone 1's yaw channel: its values start 24 bytes past its record.
        let values = animation_at + 12 + 24;
        b[animation_at + 12 + 10..animation_at + 12 + 12].copy_from_slice(&24u16.to_le_bytes());
        b.resize(values + 4, 0);
        b[values] = 1; // one value stored
        b[values + 1] = 1; // covering one frame
        b[values + 2..values + 4].copy_from_slice(&157i16.to_le_bytes());

        // Textures and the skin table.
        if with_textures {
            add_textures(&mut b);
        }

        // One body part, one model, four vertices on bone 1, one mesh.
        let part_at = b.len();
        put_int(&mut b, 204, 1);
        put_int(&mut b, 208, part_at as i32);
        b.resize(part_at + BODY_PART_SIZE, 0);
        put_int(&mut b, part_at + 64, 1);
        let model_at = b.len();
        put_int(&mut b, part_at + 72, model_at as i32);
        b.resize(model_at + MODEL_SIZE, 0);
        put_int(&mut b, model_at + 72, 1);
        put_int(&mut b, model_at + 80, 4);
        let bone_of = b.len();
        put_int(&mut b, model_at + 84, bone_of as i32);
        b.extend_from_slice(&[1, 1, 1, 1]);
        let vertex_at = b.len();
        put_int(&mut b, model_at + 88, vertex_at as i32);
        for (x, y) in [(1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (2.0, 0.0)] {
            for value in [x, y, 0.0f32] {
                b.extend_from_slice(&value.to_le_bytes());
            }
        }
        let mesh_at = b.len();
        put_int(&mut b, model_at + 76, mesh_at as i32);
        b.resize(mesh_at + MESH_SIZE, 0);
        let commands = b.len();
        put_int(&mut b, mesh_at + 4, commands as i32);
        let mut command = |count: i16, corners: &[i16]| {
            b.extend_from_slice(&count.to_le_bytes());
            for vertex in corners {
                for value in [*vertex, 0, *vertex, 1] {
                    b.extend_from_slice(&value.to_le_bytes());
                }
            }
        };
        command(4, &[0, 1, 2, 3]);
        command(-3, &[3, 2, 0]);
        command(0, &[]);
        b
    }

    fn add_textures(b: &mut Vec<u8>) {
        let texture_at = b.len();
        put_int(b, 180, 1);
        put_int(b, 184, texture_at as i32);
        b.resize(texture_at + TEXTURE_SIZE, 0);
        put_int(b, texture_at + 64, MASKED);
        put_int(b, texture_at + 68, 2);
        put_int(b, texture_at + 72, 2);
        let pixels = b.len();
        put_int(b, texture_at + 76, pixels as i32);
        b.extend_from_slice(&[1, 1, 1, 255]);
        let mut palette = vec![0u8; 768];
        palette[3..6].copy_from_slice(&[200, 100, 50]);
        b.extend_from_slice(&palette);
        let table = b.len();
        put_int(b, 192, 1);
        put_int(b, 196, 1);
        put_int(b, 200, table as i32);
        b.extend_from_slice(&0i16.to_le_bytes());
    }

    fn put_int(b: &mut [u8], at: usize, value: i32) {
        b[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_float(b: &mut [u8], at: usize, value: f32) {
        b[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn close(a: Vec3d, b: Vec3d) -> bool {
        geometry_math::vec::length(a - b) < 1e-3
    }

    #[test]
    fn vertices_are_placed_by_the_first_frame_of_their_bone() {
        let m = read(&model(true), &mut || panic!("the textures are in the file")).unwrap();
        // Bone 1 is turned a quarter about Z and sits 10 above the origin, so the
        // vertex (1, 0, 0) of its own space lands on (0, 1, 10).
        let first = m.triangles[0].positions[0];
        assert!(close(first, Vec3d::new(0.0, 1.0, 10.0)), "{first:?}");
        assert!(
            close(m.bounds.max, Vec3d::new(0.0, 2.0, 10.0)),
            "{:?}",
            m.bounds
        );
        assert!(
            close(m.bounds.min, Vec3d::new(-1.0, 0.0, 10.0)),
            "{:?}",
            m.bounds
        );
    }

    #[test]
    fn strips_alternate_their_winding_and_fans_turn_about_the_first_corner() {
        let m = read(&model(true), &mut || None).unwrap();
        assert_eq!(m.triangles.len(), 3);
        let s = |t: &ModelTriangle| t.uv.map(|uv| uv.x() as i32);
        assert_eq!(s(&m.triangles[0]), [0, 1, 2]);
        assert_eq!(s(&m.triangles[1]), [2, 1, 3]);
        assert_eq!(s(&m.triangles[2]), [3, 2, 0]);
        assert_eq!(m.triangles[0].uv[0].y(), 1.0);
    }

    #[test]
    fn a_texture_is_coloured_by_its_own_palette_and_masked_has_a_hole() {
        let m = read(&model(true), &mut || None).unwrap();
        assert_eq!((m.skin.width, m.skin.height), (2, 2));
        assert_eq!(&m.skin.pixels[..4], &[200, 100, 50, 255]);
        assert_eq!(
            m.skin.pixels[15], 0,
            "index 255 of a masked texture is a hole"
        );
    }

    #[test]
    fn textures_kept_in_the_companion_file_are_used() {
        let mut companion = vec![0u8; HEADER_SIZE];
        companion[..4].copy_from_slice(b"IDST");
        put_int(&mut companion, 4, 10);
        add_textures(&mut companion);
        let mut asked = false;
        let m = read(&model(false), &mut || {
            asked = true;
            Some(companion.clone())
        })
        .unwrap();
        assert!(asked);
        assert_eq!(&m.skin.pixels[..4], &[200, 100, 50, 255]);

        // Without it, the shape is still drawn, in grey.
        let m = read(&model(false), &mut || None).unwrap();
        assert_eq!(m.triangles.len(), 3);
        assert_eq!(m.skin.pixels, vec![160, 160, 160, 255]);
    }

    #[test]
    fn a_damaged_file_is_an_error_and_never_a_panic() {
        let whole = model(true);
        for end in 0..whole.len() {
            assert!(read(&whole[..end], &mut || None).is_err(), "cut at {end}");
        }
        let mut quake = whole.clone();
        quake[..4].copy_from_slice(b"IDPO");
        assert!(read(&quake, &mut || None).is_err());
        let mut version = whole.clone();
        put_int(&mut version, 4, 6);
        assert!(read(&version, &mut || None).is_err());
        for field in [140, 144, 180, 184, 204, 208] {
            let mut bad = whole.clone();
            put_int(&mut bad, field, -7);
            assert!(read(&bad, &mut || None).is_err(), "field {field}");
        }
    }
}
