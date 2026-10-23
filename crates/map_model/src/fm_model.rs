//! Static Heretic II flex model (`.fm`) previews: the first frame and the first skin.
//!
//! A flex model is a run of chunks, each a 32-byte name, a version and a byte size, then
//! the data. The ones a preview needs carry Quake 2's MD2 layout: `header` (skin size,
//! frame size, then the counts of skins, vertices, texture coordinates, triangles, GL
//! commands, frames and mesh nodes), `skin` (64-byte names), `st coord` (two `i16` a
//! point), `tris` (three vertex and three coordinate indices, `u16`) and `frames`
//! (scale, translation, a 16-byte name and four bytes a vertex). The rest, GL commands,
//! mesh nodes and skeletons, is skipped. Layout reference: Raven's `fmodel.h` in the
//! Heretic II toolkit.
use crate::entity_model::{EntityModel, ModelTriangle, Skin};
use geometry_math::{
    bbox::Bbox3d,
    vec::{Vec2f, Vec3d},
};

const CHUNK_HEADER: usize = 40;

/// The chunks of a flex model, by name; a later chunk of the same name is ignored.
fn chunks(bytes: &[u8]) -> Result<Vec<(&str, &[u8])>, String> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let head = bytes
            .get(at..at + CHUNK_HEADER)
            .ok_or("truncated flex model chunk")?;
        let name = head[..32].split(|b| *b == 0).next().unwrap_or_default();
        let name = std::str::from_utf8(name).map_err(|_| "unreadable flex model chunk name")?;
        let size = i32::from_le_bytes(head[36..40].try_into().expect("four bytes"));
        let size = usize::try_from(size).map_err(|_| "negative flex model chunk size")?;
        let data = at
            .checked_add(CHUNK_HEADER + size)
            .and_then(|end| bytes.get(at + CHUNK_HEADER..end))
            .ok_or("truncated flex model chunk")?;
        found.push((name, data));
        at += CHUNK_HEADER + size;
    }
    Ok(found)
}

/// Decodes a skin the model names: an `.m8` through its own palette, a `.pcx`, or any
/// image the `image` crate reads.
fn decode_skin(name: &str, bytes: Vec<u8>) -> Option<Skin> {
    let lowercase = name.to_ascii_lowercase();
    if lowercase.ends_with(".m8") {
        crate::m8::read(&bytes).ok()
    } else if lowercase.ends_with(".pcx") {
        crate::pcx::read(&bytes).ok()
    } else {
        let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
        Some(Skin {
            width: image.width(),
            height: image.height(),
            pixels: image.into_raw(),
        })
    }
}

/// Reads a flex model. `resource` resolves skin names in the game's filesystem; Raven
/// wrote them with backslashes (`models/monsters\rat/!skin.m8`), which are turned into
/// slashes first. A missing skin draws grey; malformed geometry returns an error.
pub fn read(
    bytes: &[u8],
    resource: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<EntityModel, String> {
    if bytes.get(..6) != Some(b"header") {
        return Err("not a Heretic II flex model".into());
    }
    let bad = || "invalid or truncated flex model".to_string();
    let chunks = chunks(bytes)?;
    let chunk = |name: &str| {
        chunks
            .iter()
            .find(|(found, _)| *found == name)
            .map(|(_, data)| *data)
    };
    let header = chunk("header").filter(|h| h.len() >= 40).ok_or_else(bad)?;
    let fields: Vec<usize> = header[..40]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| usize::try_from(i32::from_le_bytes(*word)).map_err(|_| bad()))
        .collect::<Result<_, _>>()?;
    let [width, height, frame_size, skins, vertices, st_count, tri_count, _, frames, _]: [usize;
        10] = fields.try_into().map_err(|_| bad())?;
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || vertices == 0
        || vertices > 65536
        || st_count == 0
        || tri_count == 0
        || frames == 0
        || frame_size < 40 + vertices * 4
    {
        return Err(bad());
    }
    let coordinates = chunk("st coord")
        .filter(|data| data.len() >= st_count * 4)
        .ok_or_else(bad)?;
    let faces = chunk("tris")
        .filter(|data| data.len() >= tri_count * 12)
        .ok_or_else(bad)?;
    let frame = chunk("frames")
        .and_then(|data| data.get(..frame_size))
        .ok_or_else(bad)?;
    let float = |at: usize| f32::from_le_bytes(frame[at..at + 4].try_into().expect("four")) as f64;
    let scale = [float(0), float(4), float(8)];
    let shift = [float(12), float(16), float(20)];
    if scale.iter().chain(&shift).any(|n| !n.is_finite()) {
        return Err("non-finite flex model frame transform".into());
    }
    let skin = chunk("skin")
        .filter(|_| skins > 0)
        .and_then(|names| names.get(..64))
        .and_then(|name| {
            let name = std::str::from_utf8(name.split(|b| *b == 0).next()?).ok()?;
            let name = name.replace('\\', "/");
            decode_skin(&name, resource(&name)?)
        })
        .unwrap_or_else(|| Skin {
            width: 1,
            height: 1,
            pixels: vec![160, 160, 160, 255],
        });
    let mut triangles = Vec::with_capacity(tri_count);
    let mut bounds: Option<Bbox3d> = None;
    for face in faces[..tri_count * 12].as_chunks::<12>().0 {
        let short = |at: usize| u16::from_le_bytes(face[at..at + 2].try_into().expect("two"));
        let mut positions = [Vec3d::zero(); 3];
        let mut uv = [Vec2f::new(0., 0.); 3];
        for corner in 0..3 {
            let index = short(corner * 2) as usize;
            let st = short(6 + corner * 2) as usize;
            if index >= vertices || st >= st_count {
                return Err("flex model triangle index out of range".into());
            }
            let packed = &frame[40 + index * 4..44 + index * 4];
            let p = Vec3d::new(
                packed[0] as f64 * scale[0] + shift[0],
                packed[1] as f64 * scale[1] + shift[1],
                packed[2] as f64 * scale[2] + shift[2],
            );
            positions[corner] = p;
            let point = Bbox3d::new(p, p);
            bounds = Some(bounds.map_or(point, |b| b.merged(&point)));
            let texel = |offset: usize| {
                i16::from_le_bytes(coordinates[offset..offset + 2].try_into().expect("two")) as f32
            };
            uv[corner] = Vec2f::new(
                (texel(st * 4) + 0.5) * skin.width as f32 / width as f32,
                (texel(st * 4 + 2) + 0.5) * skin.height as f32 / height as f32,
            );
        }
        triangles.push(ModelTriangle {
            positions,
            uv,
            material: 0,
            normals: None,
        });
    }
    Ok(EntityModel {
        triangles,
        skin,
        extra_skins: vec![],
        pbr: vec![],
        bounds: bounds.ok_or_else(bad)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(name: &str, version: i32, data: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0; 32];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        bytes.extend(version.to_le_bytes());
        bytes.extend((data.len() as i32).to_le_bytes());
        bytes.extend(data);
        bytes
    }

    fn ints(values: &[i32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    /// One triangle on an 8 x 4 skin, three vertices, one frame scaled by 2 and moved
    /// up 10, and a mesh node chunk the reader has to step over.
    fn fixture() -> Vec<u8> {
        let mut skin = vec![0; 64];
        skin[..24].copy_from_slice(b"models\\check/!skin.m8\0\0\0");
        let st: Vec<u8> = [0i16, 0, 7, 0, 0, 3]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let tris: Vec<u8> = [0u16, 1, 2, 0, 1, 2]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let mut frame = Vec::new();
        for value in [2.0f32, 2.0, 2.0, 0.0, 0.0, 10.0] {
            frame.extend(value.to_le_bytes());
        }
        frame.extend(b"stand1\0\0\0\0\0\0\0\0\0\0");
        frame.extend([0, 0, 0, 0, 4, 0, 0, 0, 0, 3, 0, 0]);
        let mut bytes = chunk("header", 2, &ints(&[8, 4, 52, 1, 3, 3, 1, 0, 1, 1]));
        bytes.extend(chunk("skin", 1, &skin));
        bytes.extend(chunk("st coord", 1, &st));
        bytes.extend(chunk("tris", 1, &tris));
        bytes.extend(chunk("frames", 1, &frame));
        bytes.extend(chunk("mesh nodes", 3, &[0; 12]));
        bytes
    }

    #[test]
    fn the_first_frame_and_the_m8_skin_make_the_preview() {
        let model = read(&fixture(), &mut |name| {
            assert_eq!(name, "models/check/!skin.m8");
            Some(crate::m8::tests::fixture())
        })
        .expect("the fixture reads");
        assert_eq!(model.triangles.len(), 1);
        assert_eq!(
            model.triangles[0].positions,
            [
                Vec3d::new(0., 0., 10.),
                Vec3d::new(8., 0., 10.),
                Vec3d::new(0., 6., 10.)
            ]
        );
        assert_eq!(model.triangles[0].uv[1], Vec2f::new(7.5, 0.5));
        assert_eq!((model.skin.width, model.skin.height), (8, 4));
        assert_eq!(model.bounds.max, Vec3d::new(8., 6., 10.));
    }

    #[test]
    fn a_missing_skin_is_grey_and_broken_files_are_errors() {
        let model = read(&fixture(), &mut |_| None).expect("geometry alone reads");
        assert_eq!(model.skin.pixels, vec![160, 160, 160, 255]);
        let source = fixture();
        for end in 0..source.len() - 52 {
            assert!(read(&source[..end], &mut |_| None).is_err(), "{end}");
        }
        let mut bad_index = source.clone();
        // The first vertex index of the triangle, past the header, skin and st chunks.
        let tris = 3 * CHUNK_HEADER + 40 + 64 + 12;
        bad_index[tris + CHUNK_HEADER..tris + CHUNK_HEADER + 2]
            .copy_from_slice(&9u16.to_le_bytes());
        assert!(read(&bad_index, &mut |_| None).is_err());
        let mut nan = source;
        let frames = tris + CHUNK_HEADER + 12;
        nan[frames + CHUNK_HEADER..frames + CHUNK_HEADER + 4]
            .copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(read(&nan, &mut |_| None).is_err());
    }
}
