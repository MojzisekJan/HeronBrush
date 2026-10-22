//! Reading a Wavefront `.obj` as the model an entity is drawn as.
//!
//! Loads OBJ models referenced by entity definitions, such as `model("props/crate.obj")`.
//!
//! Reads positions, UVs, supplied vertex normals and per-face materials. Convex polygons
//! are triangulated as fans. Material base colours and diffuse textures become skins.
//!
//! Faces, materials and textures are separate files, so the reader is handed a way to
//! fetch a sibling by name rather than opening anything itself: the caller knows whether
//! it is looking in a directory or in a `.pak`.

use geometry_math::bbox::Bbox3d;
use geometry_math::vec::{Vec2f, Vec3d};

use crate::entity_model::{EntityModel, ModelTriangle, Skin};

/// Why an `.obj` could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjError {
    /// The bytes are not text.
    NotText,
    /// Nothing in the file makes a triangle.
    NoTriangles,
}

impl std::fmt::Display for ObjError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjError::NotText => write!(out, "not a text file, so not a Wavefront .obj"),
            ObjError::NoTriangles => write!(out, "the .obj has no faces"),
        }
    }
}

impl std::error::Error for ObjError {}

/// The skin a model with no texture of its own wears: one mid-grey texel, which the
/// renderer stretches over every triangle. A model drawn in flat grey reads as "here it
/// is, untextured", where a missing skin would read as a fault.
fn plain_skin() -> Skin {
    Skin {
        width: 1,
        height: 1,
        pixels: vec![160, 160, 160, 255],
    }
}

/// Reads an `.obj`. `sibling` fetches a file named relative to it -- the `.mtl` the OBJ
/// names, and the image that names in turn.
pub fn read(
    bytes: &[u8],
    sibling: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<EntityModel, ObjError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ObjError::NotText)?;

    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut normals = Vec::new();
    let mut triangles = Vec::new();
    let mut material_names = Vec::<String>::new();
    let mut definitions = std::collections::HashMap::<String, Skin>::new();
    let mut libraries = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut words = line.split_whitespace();
        let Some(keyword) = words.next() else {
            continue;
        };
        match keyword {
            "v" | "vn" => {
                let mut number = || words.next().and_then(|w| w.parse::<f64>().ok());
                if let (Some(x), Some(y), Some(z)) = (number(), number(), number()) {
                    let p = Vec3d::new(x, y, z);
                    if keyword == "v" {
                        positions.push(p)
                    } else {
                        normals.push(p)
                    }
                }
            }
            "vt" => {
                let mut number = || words.next().and_then(|w| w.parse::<f32>().ok());
                let (u, v) = (number(), number());
                uvs.push(Vec2f::new(u.unwrap_or(0.0), v.unwrap_or(0.0)));
            }
            "usemtl" => current = words.collect::<Vec<_>>().join(" "),
            "mtllib" => libraries.push(words.collect::<Vec<_>>().join(" ")),
            "f" => {
                let corners = words
                    .map(|corner| {
                        let mut parts = corner.split('/');
                        let p = index(parts.next()?, positions.len())?;
                        let uv = parts.next().and_then(|s| index(s, uvs.len()));
                        let n = parts.next().and_then(|s| index(s, normals.len()));
                        Some((p, uv, n))
                    })
                    .collect::<Option<Vec<_>>>();
                let Some(corners) = corners.filter(|c| c.len() >= 3) else {
                    continue;
                };
                let material = material_names
                    .iter()
                    .position(|m| m == &current)
                    .unwrap_or_else(|| {
                        material_names.push(current.clone());
                        material_names.len() - 1
                    });
                for i in 1..corners.len() - 1 {
                    let picked = [corners[0], corners[i], corners[i + 1]];
                    let supplied = picked.map(|(_, _, n)| n.map(|n| normals[n]));
                    let supplied = supplied
                        .into_iter()
                        .collect::<Option<Vec<_>>>()
                        .and_then(|n| n.into_iter().map(unit_normal).collect::<Option<Vec<_>>>())
                        .map(|n| [n[0], n[1], n[2]]);
                    triangles.push(ModelTriangle {
                        positions: picked.map(|(p, _, _)| positions[p]),
                        uv: picked.map(|(_, uv, _)| uv.map_or(Vec2f::zero(), |uv| uvs[uv])),
                        material,
                        normals: supplied,
                    });
                }
            }
            _ => {}
        }
    }
    if triangles.is_empty() {
        return Err(ObjError::NoTriangles);
    }
    let mut legacy_skin = None;
    for library in libraries {
        let Some(bytes) = sibling(&library) else {
            continue;
        };
        let Some(text) = std::str::from_utf8(&bytes).ok() else {
            continue;
        };
        let directory = std::path::Path::new(&library)
            .parent()
            .unwrap_or(std::path::Path::new(""));
        let mut blocks = Vec::<(String, String)>::new();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let mut words = line.split_whitespace();
            if words.next() == Some("newmtl") {
                blocks.push((words.collect::<Vec<_>>().join(" "), String::new()));
            } else if let Some((_, block)) = blocks.last_mut() {
                block.push_str(line);
                block.push('\n');
            }
        }
        for (name, block) in blocks {
            let mut skin = first_texture(block.as_bytes())
                .and_then(|name| sibling(&directory.join(name).to_string_lossy()))
                .and_then(|bytes| decode(&bytes));
            let mut color = [1.0_f64; 4];
            let mut has_color = false;
            for line in block.lines() {
                let mut words = line.split_whitespace();
                match words.next() {
                    Some("Kd") => {
                        for channel in &mut color[..3] {
                            *channel = words.next().and_then(|w| w.parse().ok()).unwrap_or(1.0);
                        }
                        has_color = true;
                    }
                    Some("d") => {
                        color[3] = words.next().and_then(|w| w.parse().ok()).unwrap_or(1.0);
                    }
                    Some("Tr") => {
                        color[3] = 1.0
                            - words
                                .next()
                                .and_then(|w| w.parse::<f64>().ok())
                                .unwrap_or(0.0);
                    }
                    _ => {}
                }
            }
            if skin.is_none() {
                skin = Some(if has_color {
                    Skin {
                        width: 1,
                        height: 1,
                        pixels: vec![255; 4],
                    }
                } else {
                    plain_skin()
                });
            }
            let mut skin = skin.unwrap();
            for pixel in skin.pixels.as_chunks_mut::<4>().0 {
                for i in 0..4 {
                    pixel[i] = (pixel[i] as f64 * color[i].clamp(0.0, 1.0)).round() as u8;
                }
            }
            legacy_skin.get_or_insert_with(|| skin.clone());
            definitions.insert(name.trim().to_owned(), skin);
        }
    }
    let mut skins = material_names
        .iter()
        .map(|name| {
            definitions
                .get(name)
                .cloned()
                .or_else(|| name.is_empty().then(|| legacy_skin.clone()).flatten())
                .unwrap_or_else(plain_skin)
        })
        .collect::<Vec<_>>();
    for triangle in &mut triangles {
        let skin = &skins[triangle.material];
        for uv in &mut triangle.uv {
            *uv = Vec2f::new(
                uv.x() * skin.width as f32,
                (1.0 - uv.y()) * skin.height as f32,
            );
        }
    }
    let bounds = Bbox3d::merge_all(triangles.iter().flat_map(|t| t.positions)).unwrap();
    let skin = skins.remove(0);
    Ok(EntityModel {
        triangles,
        skin,
        extra_skins: skins,
        pbr: vec![],
        bounds,
    })
}

fn unit_normal(n: Vec3d) -> Option<Vec3d> {
    let length = geometry_math::vec::length(n);
    (length.is_finite() && length > 1e-12).then(|| n / length)
}

/// One index of an OBJ face corner, which counts from one, or from the end when negative.
/// An empty field, as in `f 1//1`, is nothing.
fn index(field: &str, len: usize) -> Option<usize> {
    let value = field.trim().parse::<i64>().ok()?;
    let at = if value > 0 {
        usize::try_from(value - 1).ok()?
    } else if value < 0 {
        len.checked_sub(usize::try_from(-value).ok()?)?
    } else {
        return None;
    };
    (at < len).then_some(at)
}

/// The diffuse texture named by one material block.
fn first_texture(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if let Some(rest) = line.strip_prefix("map_Kd") {
            // The keyword may be followed by options (`-s 1 1 1 file.png`); the file name
            // is what is left after them, and it may itself have spaces in it.
            let mut rest = rest.trim();
            while rest.starts_with('-') {
                let mut words = rest.splitn(2, char::is_whitespace);
                let option = words.next().unwrap_or("");
                let after = words.next().unwrap_or("").trim_start();
                // Every option this cares about takes one to three numbers.
                let numbers = after
                    .split_whitespace()
                    .take_while(|word| word.parse::<f64>().is_ok())
                    .count();
                let _ = option;
                rest = after
                    .splitn(numbers + 1, char::is_whitespace)
                    .last()
                    .unwrap_or("")
                    .trim_start();
            }
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// An image file as a skin.
fn decode(bytes: &[u8]) -> Option<Skin> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    Some(Skin {
        width: image.width(),
        height: image.height(),
        pixels: image.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit square as two triangles, with texture coordinates.
    const SQUARE: &str = "\
# a square
v 0 0 0
v 16 0 0
v 16 16 0
v 0 16 0
vt 0 0
vt 1 0
vt 1 1
vt 0 1
f 1/1 2/2 3/3 4/4
";

    fn nothing(_: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn materials_keep_their_own_dimensions_uvs_and_supplied_normals() {
        let image = image::RgbaImage::from_pixel(3, 2, image::Rgba([100, 200, 50, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let mut sibling = |name: &str| match name {
            "materials/paint.mtl" => {
                Some(b"newmtl\tred\nKd 1 0 0\nnewmtl textured\nmap_Kd image.png\n".to_vec())
            }
            "materials/image.png" => Some(png.get_ref().clone()),
            _ => None,
        };
        let text = "mtllib materials/paint.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nvt 2 -1\nvn 0 0 2\nvn 0 1 1\nusemtl red\nf 1/1/1 2/1/2 3/1/1\nusemtl textured\nf -3/1/-2 -2/1/-1 -1/1/-2\n";
        let model = read(text.as_bytes(), &mut sibling).unwrap();
        assert_eq!(model.skin.pixels, [255, 0, 0, 255]);
        assert_eq!(
            (model.extra_skins[0].width, model.extra_skins[0].height),
            (3, 2)
        );
        assert_eq!(model.triangles[1].material, 1);
        assert_eq!(model.triangles[1].uv[0], Vec2f::new(6.0, 4.0));
        let n = model.triangles[0].normals.unwrap();
        assert_eq!(n[0], Vec3d::new(0.0, 0.0, 1.0));
        assert!((n[1].y() - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
    }

    #[test]
    fn a_quad_becomes_two_triangles() {
        let model = read(SQUARE.as_bytes(), &mut nothing).expect("it reads");
        assert_eq!(model.triangles.len(), 2);
        assert_eq!(model.bounds.min, Vec3d::zero());
        assert_eq!(model.bounds.max, Vec3d::new(16.0, 16.0, 0.0));
        // With no material named, the model wears the plain skin.
        assert_eq!(model.skin, plain_skin());
    }

    #[test]
    fn a_negative_index_counts_back_from_the_last_vertex() {
        let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n";
        let model = read(text.as_bytes(), &mut nothing).expect("it reads");
        assert_eq!(model.triangles.len(), 1);
        assert_eq!(model.triangles[0].positions[0], Vec3d::zero());
        assert_eq!(model.triangles[0].positions[2], Vec3d::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn a_face_with_normals_and_no_texture_coordinates_still_reads() {
        let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvn 0 0 1\nf 1//1 2//1 3//1\n";
        let model = read(text.as_bytes(), &mut nothing).expect("it reads");
        assert_eq!(model.triangles.len(), 1);
        // A corner with no coordinate of its own gets the corner of the skin, which for
        // the plain one-texel skin is the only texel there is.
        let uv = model.triangles[0].uv;
        assert!(uv.iter().all(|corner| *corner == uv[0]), "{uv:?}");
        assert!(uv[0].x() <= model.skin.width as f32);
        assert!(uv[0].y() <= model.skin.height as f32);
    }

    #[test]
    fn a_file_with_no_faces_is_an_error() {
        assert_eq!(read(b"v 0 0 0\n", &mut nothing), Err(ObjError::NoTriangles));
        assert_eq!(read(&[0xff, 0xfe], &mut nothing), Err(ObjError::NotText));
    }

    #[test]
    fn the_texture_the_material_names_becomes_the_skin() {
        // A two-by-one PNG, so the coordinates have something to scale by.
        let png = {
            let image = image::RgbaImage::from_fn(2, 1, |x, _| {
                image::Rgba([if x == 0 { 255 } else { 0 }, 0, 0, 255])
            });
            let mut bytes = std::io::Cursor::new(Vec::new());
            image
                .write_to(&mut bytes, image::ImageFormat::Png)
                .expect("it encodes");
            bytes.into_inner()
        };
        let mut sibling = |name: &str| match name {
            "square.mtl" => Some(b"newmtl thing\nmap_Kd skin.png\n".to_vec()),
            "skin.png" => Some(png.clone()),
            _ => None,
        };
        let text = format!("mtllib square.mtl\n{SQUARE}");
        let model = read(text.as_bytes(), &mut sibling).expect("it reads");
        assert_eq!((model.skin.width, model.skin.height), (2, 1));
        // The corner at (0, 0) in the file is the top left of the skin, which is where
        // the first row starts: V is flipped and both are in texels now.
        let corners: Vec<Vec2f> = model.triangles.iter().flat_map(|t| t.uv).collect();
        assert!(corners.contains(&Vec2f::new(0.0, 1.0)), "{corners:?}");
        assert!(corners.contains(&Vec2f::new(2.0, 0.0)), "{corners:?}");
    }

    #[test]
    fn a_material_line_with_options_still_names_its_file() {
        assert_eq!(
            first_texture(b"map_Kd -s 1 1 1 -o 0 0 0 crate.png\n"),
            Some("crate.png".to_string())
        );
        assert_eq!(
            first_texture(b"map_Kd crate.png\n"),
            Some("crate.png".to_string())
        );
        assert_eq!(first_texture(b"newmtl thing\n"), None);
    }
}
