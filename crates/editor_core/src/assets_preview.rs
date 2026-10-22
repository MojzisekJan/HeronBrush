//! CPU thumbnail rendering with per-pixel visibility, also used by MCP.
use geometry_math::vec::{Vec2f, Vec3d};
use map_model::brush_face::FaceColor;

#[derive(Clone, Copy)]
struct Fragment {
    depth: f64,
    rgba: [f64; 4],
}
struct Face {
    points: Vec<[f64; 3]>,
    uv: Vec<Vec2f>,
    sampler: Option<map_model::brush_face::PaintOnFace>,
    shade: f64,
    color: Option<FaceColor>,
    texture: Option<std::sync::Arc<map_model::material::Material>>,
}

impl Face {
    fn translucent(&self) -> bool {
        self.color.map_or_else(
            || {
                self.texture
                    .as_ref()
                    .is_some_and(|m| m.alpha() != map_model::material::Alpha::Opaque)
            },
            |c| c.alpha() < 255,
        )
    }
}

fn composite(back: [f64; 4], front: [f64; 4]) -> [f64; 4] {
    let a = front[3] + back[3] * (1.0 - front[3]);
    if a == 0.0 {
        return [0.0; 4];
    }
    let mut out = [0.0; 4];
    out[3] = a;
    for i in 0..3 {
        out[i] = (front[i] * front[3] + back[i] * back[3] * (1.0 - front[3])) / a;
    }
    out
}

fn color_at(face: &Face, uv: Vec2f) -> [f64; 4] {
    let rgb = |c: map_model::brush_face::Rgb| {
        [
            f64::from(c.r) / 255.,
            f64::from(c.g) / 255.,
            f64::from(c.b) / 255.,
        ]
    };
    let (color, alpha) = match face.color {
        Some(FaceColor::Solid { color, alpha }) => (rgb(color), f64::from(alpha) / 255.),
        Some(paint) => (
            face.sampler
                .as_ref()
                .expect("painted face sampler")
                .at_uv(uv)
                .map(f64::from),
            f64::from(paint.alpha()) / 255.0,
        ),
        None => match &face.texture {
            Some(m) => {
                let x = (uv.x().floor() as i64).rem_euclid(i64::from(m.width)) as usize;
                let y = (uv.y().floor() as i64).rem_euclid(i64::from(m.height)) as usize;
                let i = (y * m.width as usize + x) * 4;
                // In memory: the preview decoded it once when it gathered its textures.
                match m.resident_pixels() {
                    Some(p) => (
                        [
                            f64::from(p[i]) / 255.,
                            f64::from(p[i + 1]) / 255.,
                            f64::from(p[i + 2]) / 255.,
                        ],
                        f64::from(p[i + 3]) / 255.,
                    ),
                    None => ([0.60, 0.63, 0.70], 1.),
                }
            }
            None => ([0.60, 0.63, 0.70], 1.),
        },
    };
    [
        color[0] * face.shade,
        color[1] * face.shade,
        color[2] * face.shade,
        alpha,
    ]
}

fn raster(face: &Face, size: usize, mut pixel: impl FnMut(usize, Fragment)) {
    if face.points.len() < 3 {
        return;
    }
    // Each convex face contributes at most once at a pixel, including fan diagonals.
    let minx = face
        .points
        .iter()
        .map(|p| p[0])
        .fold(f64::INFINITY, f64::min)
        .floor()
        .max(0.) as usize;
    let miny = face
        .points
        .iter()
        .map(|p| p[1])
        .fold(f64::INFINITY, f64::min)
        .floor()
        .max(0.) as usize;
    let maxx = (face
        .points
        .iter()
        .map(|p| p[0])
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil()
        .max(0.) as usize)
        .min(size - 1);
    let maxy = (face
        .points
        .iter()
        .map(|p| p[1])
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil()
        .max(0.) as usize)
        .min(size - 1);
    for y in miny..=maxy {
        for x in minx..=maxx {
            let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
            for i in 1..face.points.len() - 1 {
                let [a, b, c] = [face.points[0], face.points[i], face.points[i + 1]];
                let den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
                if den.abs() < 1e-12 {
                    continue;
                }
                let wa = ((b[1] - c[1]) * (px - c[0]) + (c[0] - b[0]) * (py - c[1])) / den;
                let wb = ((c[1] - a[1]) * (px - c[0]) + (a[0] - c[0]) * (py - c[1])) / den;
                let wc = 1. - wa - wb;
                if wa >= -1e-9 && wb >= -1e-9 && wc >= -1e-9 {
                    pixel(
                        y * size + x,
                        Fragment {
                            depth: wa * a[2] + wb * b[2] + wc * c[2],
                            rgba: color_at(
                                face,
                                face.uv[0] * wa as f32
                                    + face.uv[i] * wb as f32
                                    + face.uv[i + 1] * wc as f32,
                            ),
                        },
                    );
                    break;
                }
            }
        }
    }
}

fn pixels(faces: &[Face], size: usize) -> Vec<[f64; 4]> {
    let mut opaque = vec![None::<Fragment>; size * size];
    for face in faces.iter().filter(|f| !f.translucent()) {
        raster(face, size, |i, fragment| {
            if opaque[i].is_none_or(|old| fragment.depth > old.depth) {
                opaque[i] = Some(fragment);
            }
        });
    }
    let mut out: Vec<_> = opaque
        .iter()
        .map(|f| f.map_or([0.; 4], |f| f.rgba))
        .collect();
    if faces.iter().any(Face::translucent) {
        let mut translucent = vec![Vec::<Fragment>::new(); size * size];
        for face in faces.iter().filter(|f| f.translucent()) {
            raster(face, size, |i, fragment| {
                if fragment.rgba[3] > 0. && opaque[i].is_none_or(|old| fragment.depth > old.depth) {
                    translucent[i].push(fragment);
                }
            });
        }
        for (i, layers) in translucent.iter_mut().enumerate() {
            layers.sort_by(|a, b| a.depth.total_cmp(&b.depth));
            for layer in layers {
                out[i] = composite(out[i], layer.rgba);
            }
        }
    }
    out
}

pub(super) fn render(map_text: &str, size: i32) -> Option<super::Rgba> {
    render_materials(map_text, size, None)
}

/// Resolve map-relative folders and packages just as opening the map does.
pub(super) fn render_file(path: &std::path::Path, size: i32) -> Option<super::Rgba> {
    let text = std::fs::read_to_string(path).ok()?;
    let document =
        crate::document::Document::open(path, crate::world::WorldBounds::default()).ok()?;
    render_materials(&text, size, Some(document.materials()))
}

fn render_materials(
    map_text: &str,
    size: i32,
    materials: Option<&map_model::material::MaterialCollection>,
) -> Option<super::Rgba> {
    if !(16..=2048).contains(&size) {
        return None;
    }
    let map = map_model::io::map_reader::read_map(map_text, None).ok()?;
    let bounds = geometry_math::bbox::Bbox3d::new(
        Vec3d::new(-16384., -16384., -16384.),
        Vec3d::new(16384., 16384., 16384.),
    );
    let tree = map_model::node::build_tree(&map, &bounds);
    let light = geometry_math::vec::normalize(Vec3d::new(0.3, 0.62, 0.72));
    let mut textures = std::collections::HashMap::new();
    for brush in map_model::node::brushes(&tree.world) {
        for face in brush.faces() {
            if face.color.is_none() && !textures.contains_key(face.material_name.as_str()) {
                // Decoded once here, into a material of its own that holds the pixels
                // for the whole render.
                if let Some(material) = materials.and_then(|m| m.get(&face.material_name)) {
                    if let Some(pixels) = material.pixels() {
                        let texture = map_model::material::Material::rgba(
                            material.name.clone(),
                            material.path.clone(),
                            material.width,
                            material.height,
                            pixels.to_vec(),
                        );
                        textures.insert(material.name.clone(), std::sync::Arc::new(texture));
                    }
                }
            }
        }
    }
    let mut faces = Vec::new();
    let (mut minx, mut miny, mut maxx, mut maxy) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for brush in map_model::node::brushes(&tree.world) {
        for (face, polygon) in brush.geometry_faces() {
            if materials.is_some()
                && matches!(
                    face.material_name
                        .rsplit('/')
                        .next()
                        .unwrap_or("")
                        .to_ascii_lowercase()
                        .as_str(),
                    "skip" | "clip" | "origin"
                )
            {
                continue;
            }
            let texture = textures.get(face.material_name.as_str()).cloned();
            let uv = polygon
                .iter()
                .map(|p| face.uv_coords(*p))
                .collect::<Vec<_>>();
            let sampler = face
                .paint()
                .map(|paint| map_model::brush_face::PaintOnFace::from_uv(&uv, paint));
            let mut points = Vec::new();
            for p in polygon {
                let x = (p.x() - p.y()) * 0.866_025;
                let y = (p.x() + p.y()) * 0.5 - p.z();
                minx = minx.min(x);
                maxx = maxx.max(x);
                miny = miny.min(y);
                maxy = maxy.max(y);
                points.push([x, y, p.x() + p.y() + p.z()]);
            }
            // Opaque back faces cannot be visible. Transparent backs remain layers.
            if face.paint().map_or_else(
                || {
                    texture
                        .as_ref()
                        .is_none_or(|m| m.alpha() == map_model::material::Alpha::Opaque)
                },
                |c| c.alpha() == 255,
            ) && geometry_math::vec::dot(face.normal(), Vec3d::new(1., 1., 1.)) <= 0.
            {
                continue;
            }
            faces.push(Face {
                points,
                uv,
                sampler,
                shade: 0.32 + 0.68 * geometry_math::vec::dot(face.normal(), light).max(0.),
                color: face.paint(),
                texture,
            });
        }
    }
    if faces.is_empty() || maxx <= minx || maxy <= miny {
        return None;
    }
    let margin = f64::from(size) * 0.08;
    let span = f64::from(size) - 2. * margin;
    let scale = (span / (maxx - minx)).min(span / (maxy - miny));
    for face in &mut faces {
        for p in &mut face.points {
            p[0] = (p[0] - (minx + maxx) / 2.) * scale + f64::from(size) / 2.;
            p[1] = (p[1] - (miny + maxy) / 2.) * scale + f64::from(size) / 2.;
        }
    }
    let image = pixels(&faces, size as usize);
    let mut pixels = Vec::with_capacity(image.len() * 4);
    for rgba in &image {
        pixels.extend(rgba.map(|channel| (channel * 255.).round().clamp(0., 255.) as u8));
    }
    Some(super::Rgba {
        width: size as u32,
        height: size as u32,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_model::brush_face::Rgb;
    fn face(depths: [f64; 3], rgb: Rgb, alpha: u8) -> Face {
        Face {
            uv: vec![Vec2f::new(0.0, 0.0); 3],
            sampler: None,
            points: vec![
                [0., 0., depths[0]],
                [8., 0., depths[1]],
                [0., 8., depths[2]],
            ],
            shade: 1.,
            color: Some(FaceColor::solid(rgb).with_alpha(alpha)),
            texture: None,
        }
    }
    #[test]
    fn textures_repeat_at_negative_uvs_and_paint_takes_precedence() {
        let mut f = face([1.; 3], Rgb::new(0, 255, 0), 255);
        f.texture = Some(std::sync::Arc::new(map_model::material::Material::rgba(
            "test".into(),
            Default::default(),
            2,
            1,
            vec![255, 0, 0, 255, 0, 0, 255, 128],
        )));
        assert_eq!(color_at(&f, Vec2f::new(-1., 0.)), [0., 1., 0., 1.]);
        f.color = None;
        assert_eq!(color_at(&f, Vec2f::new(2., 0.)), [1., 0., 0., 1.]);
        assert_eq!(color_at(&f, Vec2f::new(-1., 0.)), [0., 0., 1., 128. / 255.]);
    }

    #[test]
    fn transparent_texture_pixels_do_not_hide_geometry_behind_them() {
        let mut front = face([2.; 3], Rgb::new(0, 0, 0), 255);
        front.color = None;
        front.texture = Some(std::sync::Arc::new(map_model::material::Material::rgba(
            "cutout".into(),
            Default::default(),
            1,
            1,
            vec![255, 0, 0, 0],
        )));
        let back = face([1.; 3], Rgb::new(0, 0, 255), 255);
        assert_eq!(pixels(&[front, back], 8)[0], [0., 0., 1., 1.]);
    }

    #[test]
    fn map_relative_textures_render_on_a_worker_thread() {
        let root =
            std::env::temp_dir().join(format!("heron-textured-preview-{}", std::process::id()));
        std::fs::create_dir_all(root.join("maps")).unwrap();
        std::fs::create_dir_all(root.join("textures")).unwrap();
        image::RgbaImage::from_pixel(8, 8, image::Rgba([240, 10, 5, 255]))
            .save(root.join("textures/red.png"))
            .unwrap();
        let mut document = crate::document::Document::new(crate::world::WorldBounds::default());
        let brush = map_model::shapes::create_cuboid(
            &document.world_bounds(),
            &geometry_math::bbox::Bbox3d::from_scalars(-32., 32.),
            "red",
        )
        .unwrap();
        document.add_brushes(vec![brush]);
        document.load_materials(root.join("textures")).unwrap();
        let path = root.join("maps/test.map");
        document.save_as(&path).unwrap();
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("../textures"));
        let pixels = std::thread::spawn(move || render_file(&path, 96).unwrap().pixels)
            .join()
            .unwrap();
        assert!(pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 0)
            .all(|p| p[0] > p[1] * 2 && p[0] > p[2] * 2));
        assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn intersecting_faces_are_resolved_per_pixel_independently_of_order() {
        let a = face([0., 8., 0.], Rgb::new(255, 0, 0), 255);
        let b = face([3., 3., 3.], Rgb::new(0, 0, 255), 255);
        let rendered = pixels(&[a, b], 8);
        assert_eq!(rendered[0], [0., 0., 1., 1.]);
        assert_eq!(rendered[5], [1., 0., 0., 1.]);
        let reverse = pixels(
            &[
                face([3., 3., 3.], Rgb::new(0, 0, 255), 255),
                face([0., 8., 0.], Rgb::new(255, 0, 0), 255),
            ],
            8,
        );
        assert_eq!(rendered, reverse);
    }
    #[test]
    fn transparent_layers_composite_in_depth_order() {
        let actual = pixels(
            &[
                face([4.; 3], Rgb::new(255, 0, 0), 128),
                face([0.; 3], Rgb::new(0, 0, 255), 255),
            ],
            8,
        );
        assert!((actual[0][0] - 128. / 255.).abs() < 1e-8);
        assert!((actual[0][2] - 127. / 255.).abs() < 1e-8);
        assert_eq!(actual[0][3], 1.);
    }
}
