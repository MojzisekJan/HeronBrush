//! Static Quake II MD2 previews, using the first frame and first external skin.
//!
//! Layout reference: id Software's Quake-2/qcommon/qfiles.h, dmdl_t.
//! https://github.com/id-Software/Quake-2/blob/master/qcommon/qfiles.h
use crate::entity_model::{EntityModel, ModelTriangle, Skin};
use geometry_math::{
    bbox::Bbox3d,
    vec::{Vec2f, Vec3d},
};

/// Reads MD2 version 8. `resource` resolves skin names in the game's filesystem.
/// Missing or unsupported skins use grey; malformed geometry returns an error.
pub fn read(
    bytes: &[u8],
    resource: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<EntityModel, String> {
    let bad = || "invalid or truncated MD2".to_string();
    if bytes.get(..4) != Some(b"IDP2") {
        return Err("not a Quake II MD2".into());
    }
    if bytes.len() < 68 {
        return Err(bad());
    }
    let int = |at: usize| i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    if int(4) != 8 {
        return Err(format!("unsupported MD2 version {}", int(4)));
    }
    let fields: Vec<usize> = (8..68)
        .step_by(4)
        .map(|at| usize::try_from(int(at)).map_err(|_| bad()))
        .collect::<Result<_, _>>()?;
    let [
        width,
        height,
        frame_size,
        skins,
        vertices,
        st_count,
        tri_count,
        commands,
        frames,
        skin_at,
        st_at,
        tri_at,
        frame_at,
        command_at,
        end,
    ]: [usize; 15] = fields.try_into().unwrap();
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || vertices == 0
        || vertices > 2048
        || st_count == 0
        || st_count > 65536
        || tri_count == 0
        || tri_count > 4096
        || skins > 32
        || frames == 0
        || frames > 512
        || frame_size < 40 + vertices * 4
        || end < 68
        || end > bytes.len()
    {
        return Err(bad());
    }
    let section = |start: usize, count: usize, stride: usize| -> Result<&[u8], String> {
        if count == 0 {
            return Ok(&bytes[..0]);
        }
        let stop = count
            .checked_mul(stride)
            .and_then(|n| start.checked_add(n))
            .ok_or_else(bad)?;
        if start < 68 || stop > end {
            return Err(bad());
        }
        bytes.get(start..stop).ok_or_else(bad)
    };
    let names = section(skin_at, skins, 64)?;
    let coordinates = section(st_at, st_count, 4)?;
    let faces = section(tri_at, tri_count, 12)?;
    let frame_data = section(frame_at, frames, frame_size)?;
    section(command_at, commands, 4)?; // Commands are unused; validate their declared bounds.
    let float = |at| f32::from_le_bytes(frame_data[at..at + 4].try_into().unwrap()) as f64;
    let scale = [float(0), float(4), float(8)];
    let shift = [float(12), float(16), float(20)];
    if scale.iter().chain(&shift).any(|n| !n.is_finite()) {
        return Err("non-finite MD2 frame transform".into());
    }
    let skin = names
        .get(..64)
        .and_then(|name| {
            let name = std::str::from_utf8(name.split(|b| *b == 0).next()?).ok()?;
            let bytes = resource(name)?;
            if name.to_ascii_lowercase().ends_with(".pcx") {
                crate::pcx::read(&bytes).ok()
            } else {
                let format = image::ImageFormat::from_path(name).ok()?;
                let mut reader =
                    image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
                let mut limits = image::Limits::default();
                limits.max_alloc = Some(64 * 1024 * 1024);
                limits.max_image_width = Some(8192);
                limits.max_image_height = Some(8192);
                reader.limits(limits);
                let image = reader.decode().ok()?.to_rgba8();
                Some(Skin {
                    width: image.width(),
                    height: image.height(),
                    pixels: image.into_raw(),
                })
            }
        })
        .unwrap_or_else(|| Skin {
            width: 1,
            height: 1,
            pixels: vec![160, 160, 160, 255],
        });
    let mut triangles = Vec::with_capacity(tri_count);
    let mut bounds: Option<Bbox3d> = None;
    for face in faces.as_chunks::<12>().0 {
        let mut positions = [Vec3d::zero(); 3];
        let mut uv = [Vec2f::new(0., 0.); 3];
        for corner in 0..3 {
            let index =
                u16::from_le_bytes(face[corner * 2..corner * 2 + 2].try_into().unwrap()) as usize;
            let st = u16::from_le_bytes(face[6 + corner * 2..8 + corner * 2].try_into().unwrap())
                as usize;
            if index >= vertices || st >= st_count {
                return Err("MD2 triangle index out of range".into());
            }
            let packed = &frame_data[40 + index * 4..44 + index * 4];
            if packed[3] >= 162 {
                return Err("MD2 normal index out of range".into());
            }
            let p = Vec3d::new(
                packed[0] as f64 * scale[0] + shift[0],
                packed[1] as f64 * scale[1] + shift[1],
                packed[2] as f64 * scale[2] + shift[2],
            );
            positions[corner] = p;
            let point = Bbox3d::new(p, p);
            bounds = Some(bounds.map_or(point, |b| b.merged(&point)));
            let texel = |offset| {
                i16::from_le_bytes(coordinates[offset..offset + 2].try_into().unwrap()) as f32
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
    const MODEL: &[u8] = include_bytes!("../../../examples/editor-checks/fixtures/md2/preview.md2");
    const SKIN: &[u8] = include_bytes!("../../../examples/editor-checks/fixtures/md2/skin.pcx");
    fn read_fixture(bytes: &[u8]) -> Result<EntityModel, String> {
        read(bytes, &mut |name| {
            assert_eq!(name, "models/check/skin.pcx");
            Some(SKIN.to_vec())
        })
    }
    #[test]
    fn first_frame_scale_translation_winding_bounds_and_texels() {
        let m = read_fixture(MODEL).unwrap();
        assert_eq!(m.triangles.len(), 6);
        assert_eq!(m.bounds.min, Vec3d::new(-32., -24., 0.));
        assert_eq!(m.bounds.max, Vec3d::new(32., 24., 64.));
        assert_eq!(
            m.triangles[0].positions,
            [
                Vec3d::new(-32., -24., 0.),
                Vec3d::new(32., -24., 0.),
                Vec3d::new(0., 0., 64.)
            ]
        );
        assert_eq!(
            m.triangles[0].uv,
            [
                Vec2f::new(0.5, 0.5),
                Vec2f::new(3.5, 0.5),
                Vec2f::new(1.5, 0.5)
            ]
        );
        assert_eq!((m.skin.width, m.skin.height), (4, 2));
        assert_eq!(&m.skin.pixels[..4], &[255, 40, 40, 255]);
    }
    #[test]
    fn missing_or_invalid_skin_preserves_geometry_and_normalized_uv() {
        for data in [None, Some(vec![1, 2, 3])] {
            let m = read(MODEL, &mut |_| data.clone()).unwrap();
            assert_eq!(m.triangles.len(), 6);
            assert_eq!(m.skin.pixels, vec![160, 160, 160, 255]);
            assert_eq!(m.triangles[0].uv[0], Vec2f::new(0.125, 0.25));
        }
    }
    #[test]
    fn skinless_models_and_omitted_unused_commands_are_valid() {
        let mut b = MODEL.to_vec();
        for at in [20, 36, 44, 60] {
            b[at..at + 4].fill(0);
        }
        let m = read(&b, &mut |_| panic!("no skin was declared")).unwrap();
        assert_eq!(m.triangles.len(), 6);
        assert_eq!(m.skin.pixels, vec![160, 160, 160, 255]);
    }
    #[test]
    fn raster_skin_dimensions_rescale_uv_and_tga_is_recognized_by_name() {
        let mut b = MODEL.to_vec();
        b[68..132].fill(0);
        b[68..76].copy_from_slice(b"skin.tga");
        let mut tga = vec![0; 18];
        tga[2] = 2;
        tga[12] = 8;
        tga[14] = 4;
        tga[16] = 24;
        tga[17] = 32;
        tga.extend([30, 20, 10].repeat(32));
        let m = read(&b, &mut |_| Some(tga.clone())).unwrap();
        assert_eq!((m.skin.width, m.skin.height), (8, 4));
        assert_eq!(&m.skin.pixels[..4], &[10, 20, 30, 255]);
        assert_eq!(m.triangles[0].uv[0], Vec2f::new(1., 1.));
    }
    #[test]
    fn malformed_headers_ranges_indices_and_nonfinite_frames_are_rejected_without_panics() {
        for end in 0..MODEL.len() {
            assert!(read_fixture(&MODEL[..end]).is_err(), "truncation {end}");
        }
        for (at, value) in [
            (4, 9),
            (8, 0),
            (12, -1),
            (16, 1),
            (20, 33),
            (24, i32::MAX),
            (28, 0),
            (32, 0),
            (36, i32::MAX),
            (40, 0),
            (44, -1),
            (48, 0),
            (52, 1),
            (56, i32::MAX),
            (60, i32::MAX),
            (64, i32::MAX),
        ] {
            let mut b = MODEL.to_vec();
            b[at..at + 4].copy_from_slice(&value.to_le_bytes());
            assert!(read_fixture(&b).is_err(), "field {at}");
        }
        let mut b = MODEL.to_vec();
        b[152..154].copy_from_slice(&5u16.to_le_bytes());
        assert!(read_fixture(&b).is_err());
        let mut b = MODEL.to_vec();
        b[158..160].copy_from_slice(&5u16.to_le_bytes());
        assert!(read_fixture(&b).is_err());
        let mut b = MODEL.to_vec();
        b[224..228].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(read_fixture(&b).is_err());
        let mut b = MODEL.to_vec();
        b[267] = 162;
        assert!(read_fixture(&b).is_err());
    }
}
