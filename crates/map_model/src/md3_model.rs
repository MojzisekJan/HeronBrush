//! Static Quake III MD3 version 15 previews, with all surfaces from frame zero.
//! Layout: https://github.com/ioquake/ioq3/blob/main/code/qcommon/qfiles.h
//! Normal encoding: id Software's Quake-III-Arena/q3map/misc_model.c.
use crate::entity_model::{EntityModel, ModelTriangle, Skin};
use geometry_math::{
    bbox::Bbox3d,
    vec::{Vec2f, Vec3d},
};

fn integer(bytes: &[u8], at: usize) -> Result<usize, String> {
    let value = bytes.get(at..at + 4).ok_or("truncated MD3 integer")?;
    usize::try_from(i32::from_le_bytes(value.try_into().unwrap()))
        .map_err(|_| "negative MD3 count or offset".into())
}
fn section(
    bytes: &[u8],
    at: usize,
    count: usize,
    stride: usize,
    minimum: usize,
) -> Result<std::ops::Range<usize>, String> {
    let end = count
        .checked_mul(stride)
        .and_then(|n| at.checked_add(n))
        .filter(|&end| end <= bytes.len())
        .ok_or("MD3 section exceeds bounds")?;
    if count != 0 && at < minimum {
        return Err("MD3 section overlaps header".into());
    }
    Ok(at..end)
}
fn disjoint(ranges: &[std::ops::Range<usize>]) -> Result<(), String> {
    for (i, a) in ranges.iter().enumerate() {
        for b in &ranges[i + 1..] {
            if !a.is_empty() && !b.is_empty() && a.start < b.end && b.start < a.end {
                return Err("overlapping MD3 sections".into());
            }
        }
    }
    Ok(())
}
fn texture(name: &str, resource: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> Option<Skin> {
    let mut paths = vec![name.to_string()];
    if std::path::Path::new(name).extension().is_none() {
        paths.extend(["tga", "jpg", "png", "jpeg"].map(|ext| format!("{name}.{ext}")));
    }
    for path in paths {
        let Some(bytes) = resource(&path) else {
            continue;
        };
        let Ok(format) = image::ImageFormat::from_path(&path) else {
            continue;
        };
        let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(64 << 20);
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        reader.limits(limits);
        if let Ok(image) = reader.decode() {
            // A grayscale decoder may fit its limit while RGBA conversion would
            // quadruple it. Bound the eventual skin before allocating RGBA.
            if u64::from(image.width()) * u64::from(image.height()) > 16_777_216 {
                continue;
            }
            let pixels = image.to_rgba8();
            return Some(Skin {
                width: pixels.width(),
                height: pixels.height(),
                pixels: pixels.into_raw(),
            });
        }
    }
    None
}

/// Reads frame zero and the first shader image of every MD3 surface.
/// Shader scripts, .skin remaps, tags and animation are not interpreted.
/// Missing raster textures use grey; malformed geometry is rejected.
pub fn read(
    bytes: &[u8],
    resource: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<EntityModel, String> {
    if bytes.get(..4) != Some(b"IDP3") {
        return Err("not a Quake III MD3".into());
    }
    if integer(bytes, 4)? != 15 {
        return Err("unsupported MD3 version".into());
    }
    if bytes.len() < 108 {
        return Err("truncated MD3 header".into());
    }
    let frames = integer(bytes, 76)?;
    let tags = integer(bytes, 80)?;
    let surfaces = integer(bytes, 84)?;
    let end = integer(bytes, 104)?;
    if !(1..=1024).contains(&frames)
        || tags > 16
        || !(1..=32).contains(&surfaces)
        || end < 108
        || end > bytes.len()
    {
        return Err("invalid MD3 header limits".into());
    }
    let bytes = &bytes[..end];
    let frame_range = section(bytes, integer(bytes, 92)?, frames, 56, 108)?;
    let tag_range = section(bytes, integer(bytes, 96)?, frames * tags, 112, 108)?;
    let mut at = integer(bytes, 100)?;
    if at < 108 || at > end {
        return Err("invalid MD3 surface offset".into());
    }
    disjoint(&[frame_range, tag_range, at..end])?;
    let mut triangles = Vec::new();
    let mut skins = Vec::new();
    let mut texture_bytes = 0usize;
    let mut bounds: Option<Bbox3d> = None;
    for material in 0..surfaces {
        let header = bytes
            .get(at..)
            .filter(|b| b.len() >= 108)
            .ok_or("truncated MD3 surface")?;
        if &header[..4] != b"IDP3" {
            return Err("invalid MD3 surface signature".into());
        }
        let surface_frames = integer(header, 72)?;
        let shaders = integer(header, 76)?;
        let vertices = integer(header, 80)?;
        let faces = integer(header, 84)?;
        let length = integer(header, 104)?;
        if surface_frames != frames
            || shaders > 256
            || !(1..=4096).contains(&vertices)
            || !(1..=8192).contains(&faces)
            || length < 108
        {
            return Err("invalid MD3 surface limits".into());
        }
        let surface = header.get(..length).ok_or("truncated MD3 surface")?;
        let face_range = section(surface, integer(surface, 88)?, faces, 12, 108)?;
        let shader_range = section(surface, integer(surface, 92)?, shaders, 68, 108)?;
        let uv_range = section(surface, integer(surface, 96)?, vertices, 8, 108)?;
        let vertex_range = section(surface, integer(surface, 100)?, vertices * frames, 8, 108)?;
        disjoint(&[
            face_range.clone(),
            shader_range.clone(),
            uv_range.clone(),
            vertex_range.clone(),
        ])?;
        let shader_name = if shaders == 0 {
            ""
        } else {
            let name = &surface[shader_range.start..shader_range.start + 64];
            std::str::from_utf8(name.split(|b| *b == 0).next().unwrap())
                .map_err(|_| "invalid MD3 shader name")?
        };
        let skin = if shader_name.is_empty() {
            None
        } else {
            texture(shader_name, resource)
        }
        .unwrap_or(Skin {
            width: 1,
            height: 1,
            pixels: vec![160, 160, 160, 255],
        });
        texture_bytes = texture_bytes
            .checked_add(skin.pixels.len())
            .filter(|&n| n <= 64 << 20)
            .ok_or("MD3 textures exceed 64 MiB decoded limit")?;
        let coordinates = &surface[uv_range];
        let packed = &surface[vertex_range];
        // UVs are shared by all frames. Validate even unused vertex coordinates.
        let mut points = Vec::with_capacity(vertices);
        for index in 0..vertices {
            let uv_at = index * 8;
            let u = f32::from_le_bytes(coordinates[uv_at..uv_at + 4].try_into().unwrap());
            let v = f32::from_le_bytes(coordinates[uv_at + 4..uv_at + 8].try_into().unwrap());
            let uv = Vec2f::new(u * skin.width as f32, v * skin.height as f32);
            if !uv.x().is_finite() || !uv.y().is_finite() {
                return Err("non-finite MD3 texture coordinates".into());
            }
            let vertex = &packed[index * 8..index * 8 + 8];
            let xyz = |i| i16::from_le_bytes(vertex[i..i + 2].try_into().unwrap()) as f64 / 64.;
            let position = Vec3d::new(xyz(0), xyz(2), xyz(4));
            let lat = vertex[7] as f64 * std::f64::consts::PI / 128.;
            let lng = vertex[6] as f64 * std::f64::consts::PI / 128.;
            let normal = Vec3d::new(lat.cos() * lng.sin(), lat.sin() * lng.sin(), lng.cos());
            points.push((position, uv, normal));
        }
        for face in surface[face_range].chunks_exact(12) {
            let mut positions = [Vec3d::zero(); 3];
            let mut uv = [Vec2f::new(0., 0.); 3];
            let mut normals = [Vec3d::zero(); 3];
            for corner in 0..3 {
                let index = integer(face, corner * 4)?;
                let &(p, st, normal) =
                    points.get(index).ok_or("MD3 triangle index out of range")?;
                positions[corner] = p;
                uv[corner] = st;
                normals[corner] = normal;
                let point = Bbox3d::new(p, p);
                bounds = Some(bounds.map_or(point, |b| b.merged(&point)));
            }
            triangles.push(ModelTriangle {
                positions,
                uv,
                normals: Some(normals),
                material,
            });
        }
        skins.push(skin);
        at = at.checked_add(length).ok_or("invalid MD3 surface length")?;
    }
    let skin = skins.remove(0);
    Ok(EntityModel {
        triangles,
        skin,
        extra_skins: skins,
        pbr: vec![],
        bounds: bounds.ok_or("MD3 has no geometry")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const MODEL: &[u8] = include_bytes!("../../../examples/editor-checks/fixtures/md3/preview.md3");
    fn fixture(bytes: &[u8]) -> Result<EntityModel, String> {
        read(bytes, &mut |name| match name {
            "models/check/skin0.tga" => Some(
                include_bytes!("../../../examples/editor-checks/fixtures/md3/skin0.tga").to_vec(),
            ),
            "models/check/skin1.tga" => Some(
                include_bytes!("../../../examples/editor-checks/fixtures/md3/skin1.tga").to_vec(),
            ),
            _ => None,
        })
    }
    #[test]
    fn all_surfaces_keep_frame_zero_textures_uvs_signed_vertices_and_normals() {
        let model = fixture(MODEL).unwrap();
        assert_eq!(model.triangles.len(), 2);
        assert_eq!(model.extra_skins.len(), 1);
        assert_eq!(&model.skin.pixels[..4], &[240, 20, 30, 255]);
        assert_eq!(&model.extra_skins[0].pixels[..4], &[10, 220, 40, 255]);
        for (i, triangle) in model.triangles.iter().enumerate() {
            assert_eq!(triangle.material, i);
            assert_eq!(
                triangle.positions,
                [
                    Vec3d::new(-1., 0., i as f64),
                    Vec3d::new(1., 0., i as f64),
                    Vec3d::new(0., 1., i as f64)
                ]
            );
            assert_eq!(
                triangle.uv,
                [Vec2f::new(0., 0.), Vec2f::new(2., 0.), Vec2f::new(0., 2.)]
            );
            let expected = if i == 0 {
                Vec3d::new(1., 0., 0.)
            } else {
                Vec3d::new(0., 1., 0.)
            };
            for normal in triangle.normals.unwrap() {
                assert!(geometry_math::vec::length(normal - expected) < 1e-12);
            }
        }
        assert_eq!(
            model.bounds,
            Bbox3d::new(Vec3d::new(-1., 0., 0.), Vec3d::new(1., 1., 1.))
        );
    }
    #[test]
    fn missing_textures_use_grey_and_uvs_scale_to_the_fallback() {
        let model = read(MODEL, &mut |_| None).unwrap();
        assert_eq!(model.skin.pixels, vec![160, 160, 160, 255]);
        assert_eq!(model.triangles[0].uv[1], Vec2f::new(1., 0.));
    }
    #[test]
    fn all_truncations_and_single_byte_mutations_are_bounded() {
        for end in 0..MODEL.len() {
            assert!(fixture(&MODEL[..end]).is_err(), "accepted truncation {end}");
        }
        for index in 0..MODEL.len() {
            let mut bytes = MODEL.to_vec();
            bytes[index] ^= 0xff;
            let _ = read(&bytes, &mut |_| None);
        }
    }
    #[test]
    fn invalid_indices_nonfinite_uvs_overlaps_versions_and_limits_are_rejected() {
        let surface = integer(MODEL, 100).unwrap();
        for (at, value) in [
            (4, 14),
            (76, 1025),
            (80, 17),
            (84, 33),
            (surface + 72, 1),
            (surface + 80, 4097),
            (surface + 84, 8193),
            (surface + 88, 4),
            (surface + 96, 108),
            (surface + 108, 3),
            (surface + 188, f32::NAN.to_bits()),
        ] {
            let mut bytes = MODEL.to_vec();
            bytes[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
            assert!(
                read(&bytes, &mut |_| None).is_err(),
                "accepted field at {at}"
            );
        }
    }
}
