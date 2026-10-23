//! Bake face paint to portable albedo images without changing the editor document.
use super::{Contents, Error, Options};
use crate::document::Document;
use geometry_math::vec::{Vec2f, Vec3d};
use map_model::brush_face::{BrushFace, FaceColor};
use map_model::material::Material;
use std::collections::HashMap;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

pub struct BakedFace {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    /// Affine transform from the original face UV to pixel coordinates in the bake, a row
    /// `[per u, per v, constant]` for each pixel axis.
    pub to_pixel: [[f32; 3]; 2],
    /// The pixels as PNG, when already encoded: a shared gradient image is encoded once.
    pub encoded: Option<std::sync::Arc<[u8]>>,
}
impl BakedFace {
    pub fn uv(&self, original: Vec2f) -> Vec2f {
        let [x, y] = pixel_at(self.to_pixel, original);
        Vec2f::new(x / self.width as f32, y / self.height as f32)
    }
    pub fn png(&self) -> Result<Vec<u8>, std::io::Error> {
        if let Some(png) = &self.encoded {
            return Ok(png.to_vec());
        }
        crate::assets::encode_png(self.width, self.height, &self.pixels)
            .map_err(std::io::Error::other)
    }
}

pub fn bake(face: &BrushFace, polygon: &[Vec3d], material: Option<&Material>) -> Option<BakedFace> {
    let paint = face.paint()?;
    if let FaceColor::Solid { color, alpha: 255 } = paint {
        let rgb = color.to_f32();
        return Some(BakedFace {
            width: 1,
            height: 1,
            pixels: vec![
                (rgb[0] * 255.0).round() as u8,
                (rgb[1] * 255.0).round() as u8,
                (rgb[2] * 255.0).round() as u8,
                255,
            ],
            to_pixel: CENTRE,
            encoded: None,
        });
    }
    if polygon.len() < 3 {
        return None;
    }
    let sampler = map_model::brush_face::PaintOnFace::new(face, polygon, paint);
    let underneath = material.is_some_and(|m| m.width > 0 && m.height > 0);
    if paint.alpha() == 255 || !underneath {
        let uvs: Vec<Vec2f> = polygon.iter().map(|p| face.uv_coords(*p)).collect();
        if let Some(shared) = shared_gradient(&sampler, paint, &uvs) {
            return Some(shared);
        }
    }
    let original: Vec<[f32; 2]> = polygon
        .iter()
        .map(|p| {
            let uv = face.uv_coords(*p);
            [uv.x(), uv.y()]
        })
        .collect();
    let lo: [f32; 2] =
        std::array::from_fn(|i| original.iter().map(|p| p[i]).fold(f32::INFINITY, f32::min));
    let hi: [f32; 2] = std::array::from_fn(|i| {
        original
            .iter()
            .map(|p| p[i])
            .fold(f32::NEG_INFINITY, f32::max)
    });
    let span = [(hi[0] - lo[0]).max(1e-6), (hi[1] - lo[1]).max(1e-6)];
    let width = span[0].ceil().clamp(64.0, 1024.0) as u32;
    let height = span[1].ceil().clamp(64.0, 1024.0) as u32;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    let alpha = paint.alpha() as f32 / 255.0;
    let texels = material.and_then(|m| m.pixels());
    // The first and last rows and columns lie on the face's edges, which are often
    // texel boundaries: the far edge would take the next tile's texel. Sampling the
    // texture a quarter of a bake pixel inside the face keeps them on it.
    let inset = [
        span[0] / (width - 1) as f32 / 4.0,
        span[1] / (height - 1) as f32 / 4.0,
    ];
    for y in 0..height {
        for x in 0..width {
            let uv = [
                lo[0] + span[0] * x as f32 / (width - 1) as f32,
                lo[1] + span[1] * y as f32 / (height - 1) as f32,
            ];
            let color = sampler.at_uv(Vec2f::new(uv[0], uv[1]));
            // Without an underlying image, paint alpha is surface transparency.
            // Textured paint retains the existing overlay behaviour.
            if material.is_none_or(|m| m.width == 0 || m.height == 0) {
                for c in color {
                    pixels.push((c * 255.0).round() as u8);
                }
                pixels.push(paint.alpha());
                continue;
            }
            let base = material
                .filter(|m| m.width > 0 && m.height > 0)
                .zip(texels.as_deref())
                .map(|(m, texels)| {
                    let [u, v] = [0, 1].map(|i| uv[i].clamp(lo[i] + inset[i], hi[i] - inset[i]));
                    let px = (u.floor() as i64).rem_euclid(m.width as i64) as usize;
                    let py = (v.floor() as i64).rem_euclid(m.height as i64) as usize;
                    &texels[(py * m.width as usize + px) * 4..][..4]
                })
                .unwrap_or(&[255, 255, 255, 255]);
            for c in 0..3 {
                pixels.push(
                    (color[c] * 255.0 * alpha + base[c] as f32 * (1.0 - alpha)).round() as u8,
                );
            }
            pixels.push((255.0 * alpha + base[3] as f32 * (1.0 - alpha)).round() as u8);
        }
    }
    let scale = [(width - 1) as f32 / span[0], (height - 1) as f32 / span[1]];
    Some(BakedFace {
        width,
        height,
        pixels,
        to_pixel: [
            [scale[0], 0.0, 0.5 - lo[0] * scale[0]],
            [0.0, scale[1], 0.5 - lo[1] * scale[1]],
        ],
        encoded: None,
    })
}

/// Every UV on the middle of a one-pixel image.
const CENTRE: [[f32; 3]; 2] = [[0.0, 0.0, 0.5], [0.0, 0.0, 0.5]];

/// UV unchanged: pixel coordinates of the image the face already wears.
const SAME: [[f32; 3]; 2] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

/// Where `to_pixel` takes `uv`.
fn pixel_at(to_pixel: [[f32; 3]; 2], uv: Vec2f) -> [f32; 2] {
    to_pixel.map(|row| row[0] * uv.x() + row[1] * uv.y() + row[2])
}

/// How many pixels a shared linear gradient image has along it, and a radial one each way.
const LINEAR_PIXELS: u32 = 256;
const RADIAL_PIXELS: u32 = 128;

/// A gradient as an image every face with the same colours shares, rather than one baked
/// for the face: a strip of its colours for a linear gradient, a disc of them for a radial
/// one, with the face's UVs mapped onto it. A linear gradient's colour changes evenly
/// across a face and a radial one's with the distance from an affine centre, so a mapping
/// of the corners carries both exactly. A thousand faces with one gradient then make one
/// image and one material instead of a thousand.
///
/// `None` where the face reaches past the image (a gradient placed on part of it), so it
/// is baked instead: there the colour holds at the ends, which a repeating sampler, as
/// OBJ has, would not keep.
fn shared_gradient(
    sampler: &map_model::brush_face::PaintOnFace,
    paint: FaceColor,
    corners: &[Vec2f],
) -> Option<BakedFace> {
    let [x, y] = sampler.gradient_frame()?;
    let radial = matches!(paint, FaceColor::MultiGradient { radial: true, .. });
    let alpha = paint.alpha();
    // The image depends only on the colours, so faces after the first take it as made.
    let colours = match paint {
        FaceColor::MultiGradient { stops, .. } => format!("{stops:?}"),
        FaceColor::Gradient { from, to, .. } => format!("{from:?}{to:?}"),
        FaceColor::Solid { .. } => return None,
    };
    let key = format!("{radial}/{alpha}/{colours}");
    let colour = |t: f32| {
        let c = sampler.at_position(t);
        [
            (c[0] * 255.0).round() as u8,
            (c[1] * 255.0).round() as u8,
            (c[2] * 255.0).round() as u8,
            alpha,
        ]
    };
    const EDGE: f32 = 1e-3;
    if radial {
        // Pixel centres sample the disc from -1 to 1 each way, so (x, y) maps onto the
        // pixels by a half and a shift.
        let n = RADIAL_PIXELS as f32;
        let to_pixel = [x, y].map(|row| row.map(|v| v * n / 2.0));
        let to_pixel = [
            [to_pixel[0][0], to_pixel[0][1], to_pixel[0][2] + n / 2.0],
            [to_pixel[1][0], to_pixel[1][1], to_pixel[1][2] + n / 2.0],
        ];
        let inside = corners.iter().all(|uv| {
            pixel_at(to_pixel, *uv)
                .iter()
                .all(|p| (-EDGE..=n + EDGE).contains(p))
        });
        if !inside {
            return None;
        }
        let (pixels, encoded) = shared_image(&key, RADIAL_PIXELS, RADIAL_PIXELS, || {
            let mut pixels = Vec::with_capacity((RADIAL_PIXELS * RADIAL_PIXELS * 4) as usize);
            for j in 0..RADIAL_PIXELS {
                for i in 0..RADIAL_PIXELS {
                    let at = |k: u32| (k as f32 + 0.5) / n * 2.0 - 1.0;
                    pixels.extend(colour((at(i) * at(i) + at(j) * at(j)).sqrt()));
                }
            }
            pixels
        })?;
        return Some(BakedFace {
            width: RADIAL_PIXELS,
            height: RADIAL_PIXELS,
            pixels,
            to_pixel,
            encoded: Some(encoded),
        });
    }
    // Pixel centres sample positions 0 to 1, the first and last on the gradient's ends.
    // The strip is one pixel tall; `y` runs across it so the mapping stays square.
    let n = (LINEAR_PIXELS - 1) as f32;
    let to_pixel = [
        [x[0] * n, x[1] * n, x[2] * n + 0.5],
        [y[0] * n, y[1] * n, y[2] * n + 0.5],
    ];
    let inside = corners
        .iter()
        .all(|uv| (-EDGE..=1.0 + EDGE).contains(&(x[0] * uv.x() + x[1] * uv.y() + x[2])));
    if !inside {
        return None;
    }
    let (pixels, encoded) = shared_image(&key, LINEAR_PIXELS, 1, || {
        (0..LINEAR_PIXELS)
            .flat_map(|i| colour(i as f32 / n))
            .collect()
    })?;
    Some(BakedFace {
        width: LINEAR_PIXELS,
        height: 1,
        pixels,
        to_pixel,
        encoded: Some(encoded),
    })
}

/// A shared gradient image: its pixels and their PNG.
type SharedImage = (Vec<u8>, std::sync::Arc<[u8]>);

thread_local! {
    /// Shared gradient images by their colours.
    static GRADIENTS: std::cell::RefCell<HashMap<String, SharedImage>> =
        std::cell::RefCell::new(HashMap::new());
}

/// The shared image under `key`, made by `make` and encoded the first time it is asked for.
fn shared_image(
    key: &str,
    width: u32,
    height: u32,
    make: impl FnOnce() -> Vec<u8>,
) -> Option<SharedImage> {
    if let Some(made) = GRADIENTS.with(|cache| cache.borrow().get(key).cloned()) {
        return Some(made);
    }
    let pixels = make();
    let png: std::sync::Arc<[u8]> = crate::assets::encode_png(width, height, &pixels)
        .ok()?
        .into();
    GRADIENTS.with(|cache| {
        let mut cache = cache.borrow_mut();
        // A handful of gradients is usual; a map with very many does not keep them all.
        if cache.len() > 256 {
            cache.clear();
        }
        cache.insert(key.to_string(), (pixels.clone(), png.clone()));
    });
    Some((pixels, png))
}

pub fn image_name(png: &[u8]) -> String {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    // Slice Hash includes a pointer-sized length, which differs between desktop
    // and wasm32. Hash the bytes directly so both exports name the same image.
    hash.write(png);
    format!("bbpaint_{:016x}", hash.finish())
}

/// A material's companion normal map as PNG bytes in the glTF convention: OpenGL
/// +Y, so a DirectX (`_normal_dx`) map has its green channel inverted here. Alpha
/// is set opaque; normal mapping ignores it.
pub(crate) fn normal_png(material: &Material) -> Option<std::sync::Arc<[u8]>> {
    let map = material.normal_map.as_ref()?;
    let pixels = map.pixels()?;
    let key = format!(
        "normal/{:?}/{}/{}/{}/{:?}",
        map.path,
        map.width,
        map.height,
        map.flip_y,
        std::sync::Arc::as_ptr(&pixels) as *const u8 as usize
    );
    if let Some(png) = IMAGES.with(|cache| {
        cache
            .borrow()
            .entries
            .get(&key)
            .map(|e| e.image.png.clone())
    }) {
        return Some(png.into());
    }
    let mut rgba = pixels[..(map.width * map.height * 4) as usize].to_vec();
    for pixel in rgba.as_chunks_mut::<4>().0 {
        if map.flip_y {
            pixel[1] = 255 - pixel[1];
        }
        pixel[3] = 255;
    }
    let png = crate::assets::encode_png(map.width, map.height, &rgba).ok()?;
    let size = png.len() + pixels.len() + key.len();
    IMAGES.with(|cache| {
        let mut cache = cache.borrow_mut();
        const LIMIT: usize = 64 * 1024 * 1024;
        if size <= LIMIT {
            if cache.bytes + size > LIMIT {
                cache.entries.clear();
                cache.bytes = 0;
            }
            cache.bytes += size;
            cache.entries.insert(
                key,
                CachedImage {
                    image: PreparedImage {
                        width: map.width,
                        height: map.height,
                        to_pixel: SAME,
                        alpha: false,
                        png: png.clone(),
                    },
                    _pixels: Some(pixels),
                },
            );
        }
    });
    Some(png.into())
}

#[derive(Clone)]
struct PreparedImage {
    width: u32,
    height: u32,
    to_pixel: [[f32; 3]; 2],
    alpha: bool,
    png: Vec<u8>,
}
struct CachedImage {
    image: PreparedImage,
    _pixels: Option<std::sync::Arc<[u8]>>,
}
#[derive(Default)]
struct ImageCache {
    epoch: Option<(u64, u64)>,
    bytes: usize,
    entries: HashMap<String, CachedImage>,
}
thread_local! { static IMAGES: std::cell::RefCell<ImageCache> = std::cell::RefCell::new(ImageCache::default()); }
fn prepare_image(
    face: &BrushFace,
    polygon: Option<&[Vec3d]>,
    material: Option<&Material>,
) -> Result<PreparedImage, std::io::Error> {
    let needs_base = face
        .color
        .as_deref()
        .is_none_or(|paint| paint.alpha() < 255);
    let material = if needs_base { material } else { None };
    let pixels = material.and_then(Material::pixels);
    let polygon_key = if matches!(
        face.color.as_deref(),
        Some(FaceColor::Solid { alpha: 255, .. })
    ) {
        None
    } else {
        polygon
    };
    let pixel_id = pixels
        .as_ref()
        .map(|p| std::sync::Arc::as_ptr(p) as *const u8 as usize);
    let key = format!(
        "{:?}/{:?}/{:?}/{:?}",
        face.color,
        material.map(|m| (&m.path, m.width, m.height)),
        pixel_id,
        polygon_key.map(|points| points
            .iter()
            .map(|p| face.uv_coords(*p))
            .collect::<Vec<_>>())
    );
    if let Some(image) =
        IMAGES.with(|cache| cache.borrow().entries.get(&key).map(|e| e.image.clone()))
    {
        return Ok(image);
    }
    let baked = if face.color.is_none() {
        material
            .and_then(|m| {
                Some(BakedFace {
                    width: m.width,
                    height: m.height,
                    pixels: pixels.as_ref()?[..(m.width * m.height * 4) as usize].to_vec(),
                    to_pixel: SAME,
                    encoded: None,
                })
            })
            .unwrap_or(BakedFace {
                width: 1,
                height: 1,
                pixels: vec![190, 190, 190, 255],
                to_pixel: CENTRE,
                encoded: None,
            })
    } else {
        bake(
            face,
            polygon.ok_or_else(|| std::io::Error::other("painted face has no polygon"))?,
            material,
        )
        .ok_or_else(|| std::io::Error::other("could not bake face paint"))?
    };
    let image = PreparedImage {
        width: baked.width,
        height: baked.height,
        to_pixel: baked.to_pixel,
        alpha: baked.pixels.as_chunks::<4>().0.iter().any(|p| p[3] < 255),
        png: baked.png()?,
    };
    let size = image.png.len() + pixels.as_ref().map_or(0, |p| p.len()) + key.len();
    IMAGES.with(|cache| {
        let mut cache = cache.borrow_mut();
        const LIMIT: usize = 64 * 1024 * 1024;
        if size <= LIMIT {
            if cache.bytes + size > LIMIT {
                cache.entries.clear();
                cache.bytes = 0;
            }
            cache.bytes += size;
            cache.entries.insert(
                key,
                CachedImage {
                    image: image.clone(),
                    _pixels: pixels,
                },
            );
        }
    });
    Ok(image)
}

/// Tells the Godot scene where `name`'s normal map is: the editor's own image when it
/// is already in the project and in the OpenGL convention Godot reads, otherwise the
/// converted `png` (a DirectX map's green flipped), returned to be written beside the
/// face paint.
fn godot_normal_map(
    material: &Material,
    name: &str,
    png: &[u8],
    directory: &Path,
    options: &Options,
    settings: &mut scene_export::settings::Settings,
) -> Result<Option<(PathBuf, Contents)>, Error> {
    let Some(map) = material.normal_map.as_ref() else {
        return Ok(None);
    };
    let root = settings
        .project_root
        .clone()
        .unwrap_or_else(|| directory.to_path_buf());
    let in_project = (!map.flip_y && map.path.is_file())
        .then(|| {
            let path = map.path.canonicalize().ok()?;
            scene_export::materials::fs_to_res(&root.canonicalize().ok()?, &path)
        })
        .flatten();
    if let Some(resource) = in_project {
        settings.normal_map_paths.insert(name.to_owned(), resource);
        return Ok(None);
    }
    let path = directory
        .join(format!("{}_paint", options.name))
        .join(format!("{name}_normal.png"));
    let resource = scene_export::materials::fs_to_res(&root, &path).ok_or_else(|| {
        Error::Map("export texture must be inside the chosen engine project".into())
    })?;
    settings.normal_map_paths.insert(name.to_owned(), resource);
    Ok(Some((path, Contents::Bytes(png.to_vec()))))
}

pub fn prepare(
    document: &Document,
    directory: &Path,
    options: &Options,
    settings: &mut scene_export::settings::Settings,
) -> Result<(String, Vec<(PathBuf, Contents)>), Error> {
    IMAGES.with(|cache| {
        let mut cache = cache.borrow_mut();
        let epoch = (document.generation(), document.materials_revision());
        if cache.epoch != Some(epoch) {
            cache.entries.clear();
            cache.bytes = 0;
            cache.epoch = Some(epoch);
        }
    });
    prepare_text(
        &document.to_map_text(),
        &document.world_bounds(),
        document.materials(),
        directory,
        options,
        settings,
    )
}

pub fn prepare_text(
    text: &str,
    world_bounds: &geometry_math::bbox::Bbox3d,
    materials: &map_model::material::MaterialCollection,
    directory: &Path,
    options: &Options,
    settings: &mut scene_export::settings::Settings,
) -> Result<(String, Vec<(PathBuf, Contents)>), Error> {
    let text = text.to_string();
    if options.target == super::Target::Obj {
        return Ok((text, Vec::new()));
    }
    let mut map =
        map_model::io::map_reader::read_map(&text, None).map_err(|e| Error::Map(e.to_string()))?;
    // Reuse the exporter's skip-face handling without changing the saved material.
    let mut hidden = false;
    for face in map
        .entities
        .iter_mut()
        .flat_map(|e| &mut e.brushes)
        .flat_map(|b| &mut b.faces)
    {
        if face.export_hidden {
            face.material_name = scene_export::settings::EXPORT_HIDDEN_TEXTURE.into();
            face.color = None;
            face.surface_material = None;
            hidden = true;
        }
    }
    let text = if hidden {
        map_model::io::map_writer::write_map(&map)
    } else {
        text
    };
    let mut files = Vec::new();
    // The editor's companion normal maps, for the GLB writers to embed.
    let names = map.entities.iter().flat_map(|e| {
        e.brushes
            .iter()
            .flat_map(|b| b.faces.iter().map(|f| f.material_name.as_str()))
            .chain(e.patches.iter().map(|p| p.material_name.as_str()))
    });
    for name in names {
        // Game/mod winners can live in archives or outside the exporter's own search
        // roots. Export the same pixels the editor used, including on native platforms.
        if !settings.is_tool_texture(name)
            && !settings.generated_materials.contains_key(name)
            && (cfg!(target_arch = "wasm32")
                || materials.get(name).is_some_and(Material::is_game_asset))
        {
            if let Some(material) = materials.get(name) {
                if let Some(pixels) = material.pixels() {
                    let png = crate::assets::encode_png(material.width, material.height, &pixels)
                        .map_err(Error::Map)?;
                    let path = directory
                        .join(format!("{}_paint", options.name))
                        .join(format!("{name}.png"));
                    let resource = scene_export::materials::fs_to_res(
                        settings.project_root.as_deref().unwrap_or(directory),
                        &path,
                    )
                    .ok_or_else(|| {
                        Error::Map("export texture must be inside the chosen engine project".into())
                    })?;
                    files.push((path, Contents::Bytes(png.clone())));
                    settings.generated_materials.insert(
                        name.to_owned(),
                        scene_export::materials::GeneratedMaterial {
                            pbr: None,
                            path: resource,
                            width: material.width,
                            height: material.height,
                            png,
                            alpha: pixels
                                .as_chunks::<4>()
                                .0
                                .iter()
                                .any(|pixel| pixel[3] != 255),
                            surface: None,
                            repeat: true,
                        },
                    );
                }
            }
        }
        if settings.normal_maps.contains_key(name) || settings.is_tool_texture(name) {
            continue;
        }
        if let Some(material) = materials.get(name) {
            if let Some(png) = normal_png(material) {
                if options.target == super::Target::Godot {
                    files.extend(godot_normal_map(
                        material, name, &png, directory, options, settings,
                    )?);
                }
                settings.normal_maps.insert(name.to_string(), png);
            }
        }
    }
    let unity = options.target == super::Target::Unity;
    if !unity
        && !map
            .entities
            .iter()
            .flat_map(|e| &e.brushes)
            .flat_map(|b| &b.faces)
            .any(|f| {
                (f.color.is_some() || f.surface_material.is_some())
                    && !settings.is_tool_texture(&f.material_name)
            })
    {
        return Ok((text, files));
    }
    let mut cache: HashMap<Vec<u8>, String> = HashMap::new();
    // Explicit axes preserve both original mapping and the gradient direction on
    // rotated/sloping faces. This conversion is only for the temporary export map.
    for entity in &mut map.entities {
        for source in &mut entity.brushes {
            let needs_geometry = source.faces.iter().any(|f| {
                (f.color.is_some() || f.surface_material.is_some())
                    && !settings.is_tool_texture(&f.material_name)
            });
            let geometry = if needs_geometry {
                Some(
                    map_model::brush::Brush::from_faces(world_bounds, source.faces.clone())
                        .map_err(|e| {
                            Error::Map(format!("could not prepare painted brush: {e:?}"))
                        })?,
                )
            } else {
                None
            };
            for face in &mut source.faces {
                let (u, v) = face
                    .uv_coord_system
                    .axes(face.plane.normal, &face.uv_attributes);
                face.uv_coord_system = map_model::uv::UvCoordSystem::Parallel {
                    u_axis: u,
                    v_axis: v,
                };
                if settings.is_tool_texture(&face.material_name)
                    || (face.color.is_none()
                        && face.surface_material.is_none()
                        && (!unity || materials.get(&face.material_name).is_none()))
                {
                    continue;
                }
                let material = materials.get(&face.material_name);
                let polygon = if face.color.is_some() {
                    let geometry = geometry.as_ref().expect("painted brush geometry");
                    let index = geometry
                        .faces()
                        .iter()
                        .position(|candidate| candidate.plane == face.plane)
                        .ok_or_else(|| {
                            Error::Map("painted face missing from brush geometry".into())
                        })?;
                    Some(
                        geometry
                            .face_polygon(index)
                            .ok_or_else(|| Error::Map("painted face has no polygon".into()))?,
                    )
                } else {
                    None
                };
                let image = prepare_image(face, polygon.as_deref(), material)
                    .map_err(|e| Error::Write(directory.to_path_buf(), e))?;
                let png = image.png.clone();
                let mut key = png.clone();
                if let Some(s) = face.surface_material {
                    key.extend([1, s.roughness, s.metallic]);
                    key.extend(s.emission.to_le_bytes());
                } else {
                    key.push(0);
                }
                let name = if let Some(name) = cache.get(&key) {
                    name.clone()
                } else {
                    let mut name = image_name(&key);
                    while settings.generated_materials.contains_key(&name) {
                        name.push('_');
                    }
                    let path = directory
                        .join(format!("{}_paint", options.name))
                        .join(format!("{name}.png"));
                    let root = settings.project_root.as_deref().unwrap_or(directory);
                    let resource =
                        scene_export::materials::fs_to_res(root, &path).ok_or_else(|| {
                            Error::Map(
                                "export directory must be inside the chosen engine project".into(),
                            )
                        })?;
                    settings.generated_materials.insert(
                        name.clone(),
                        scene_export::materials::GeneratedMaterial {
                            pbr: None,
                            path: resource,
                            width: image.width,
                            height: image.height,
                            alpha: image.alpha,
                            png: png.clone(),
                            surface: face.surface_material.map(|s| s.factors()),
                            // A texture face with only a surface material keeps its tiling.
                            repeat: face.color.is_none() && material.is_some(),
                        },
                    );
                    if let Some((material, png)) = material
                        .filter(|_| face.color.is_none())
                        .and_then(|m| normal_png(m).map(|png| (m, png)))
                    {
                        if options.target == super::Target::Godot {
                            files.extend(godot_normal_map(
                                material, &name, &png, directory, options, settings,
                            )?);
                        }
                        settings.normal_maps.insert(name.clone(), png);
                    }
                    files.push((path, Contents::Bytes(png.clone())));
                    cache.insert(key, name.clone());
                    name
                };
                // The face's UV is (p . u / su, p . v / sv) + offset; the image's pixel is
                // `to_pixel` of that, so each pixel axis is a mix of u and v.
                let su = f64::from(map_model::uv::safe_scale(face.uv_attributes.scale.x()));
                let sv = f64::from(map_model::uv::safe_scale(face.uv_attributes.scale.y()));
                let [row_u, row_v] = image.to_pixel.map(|row| row.map(f64::from));
                let axis = |row: [f64; 3]| u * (row[0] / su) + v * (row[1] / sv);
                face.uv_coord_system = map_model::uv::UvCoordSystem::Parallel {
                    u_axis: axis(row_u),
                    v_axis: axis(row_v),
                };
                let [x, y] = pixel_at(image.to_pixel, face.uv_attributes.offset);
                face.uv_attributes.offset = Vec2f::new(x, y);
                face.uv_attributes.scale = Vec2f::new(1.0, 1.0);
                face.uv_attributes.rotation = 0.0;
                face.material_name = name.into();
                face.color = None;
            }
        }
    }
    map.format = map_model::map_format::MapFormat::Valve;
    Ok((map_model::io::map_writer::write_map(&map), files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::bbox::Bbox3d;
    use map_model::brush_face::Rgb;

    #[test]
    fn baked_uvs_match_rotated_faces_and_unpainted_mapping_survives() {
        let mut document = Document::new(crate::world::WorldBounds::default());
        let brush = map_model::shapes::create_cuboid(
            &document.world_bounds(),
            &Bbox3d::new(Vec3d::new(128.0, -64.0, 32.0), Vec3d::new(256.0, 0.0, 80.0)),
            "stone",
        )
        .unwrap();
        document.add_brushes(vec![brush]);
        document.select_all();
        document.rotate_selection(Vec3d::new(1.0, 0.0, 0.0), 30.0);
        let id = *document.selection_ids().iter().next().unwrap();
        document.set_face_color_on(
            &[crate::document::FaceRef { node: id, face: 0 }],
            Some(FaceColor::gradient(
                Rgb::new(255, 0, 0),
                Rgb::new(0, 0, 255),
                135.0,
            )),
        );
        let directory = Path::new("/tmp/paint-uv-check");
        let options = Options::default();
        let mut settings = options.settings(directory);
        let (text, _) = prepare(&document, directory, &options, &mut settings).unwrap();
        let exported = map_model::io::map_reader::read_map(&text, None).unwrap();
        let original = document.brush(id).unwrap();
        let faces = &exported.entities[0].brushes[0].faces;
        for (i, face) in original.faces().iter().enumerate() {
            let target = faces
                .iter()
                .min_by(|a, b| {
                    geometry_math::vec::length(a.plane.normal - face.plane.normal).total_cmp(
                        &geometry_math::vec::length(b.plane.normal - face.plane.normal),
                    )
                })
                .unwrap();
            let polygon = original.face_polygon(i).unwrap();
            let baked = bake(face, &polygon, None);
            for point in &polygon {
                let expected = baked
                    .as_ref()
                    .map(|p| p.uv(face.uv_coords(*point)))
                    .unwrap_or_else(|| face.uv_coords(*point));
                let mut actual = target.uv_coords(*point);
                if let Some(image) = settings
                    .generated_materials
                    .get(target.material_name.as_str())
                {
                    actual = Vec2f::new(
                        actual.x() / image.width as f32,
                        actual.y() / image.height as f32,
                    );
                }
                assert!(
                    geometry_math::vec::length(actual - expected) < 0.001,
                    "face {i}: {actual:?} != {expected:?}"
                );
            }
        }
    }

    #[test]
    fn translucent_paint_blends_over_texture_and_skip_is_not_replaced() {
        let mut face = map_model::shapes::create_cuboid(
            &Bbox3d::new(Vec3d::fill(-1024.0), Vec3d::fill(1024.0)),
            &Bbox3d::new(Vec3d::zero(), Vec3d::fill(64.0)),
            "stone",
        )
        .unwrap()
        .faces()[0]
            .clone();
        face.set_paint(Some(FaceColor::Solid {
            color: Rgb::new(255, 0, 0),
            alpha: 128,
        }));
        let base = Material::rgba("stone".into(), PathBuf::new(), 1, 1, vec![0, 0, 255, 255]);
        let image = bake(&face, &face.points, Some(&base)).unwrap();
        assert!(image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [128, 0, 127, 255]));
        let mut document = Document::new(crate::world::WorldBounds::default());
        let brush = map_model::shapes::create_cuboid(
            &document.world_bounds(),
            &Bbox3d::new(Vec3d::zero(), Vec3d::fill(64.0)),
            "skip",
        )
        .unwrap();
        document.add_brushes(vec![brush]);
        document.select_all();
        document.set_face_color(Some(FaceColor::solid(Rgb::new(255, 0, 0))));
        let mut settings = scene_export::settings::Settings::default();
        let (text, images) = prepare(
            &document,
            Path::new("/tmp/paint-skip-check"),
            &Options::default(),
            &mut settings,
        )
        .unwrap();
        assert!(images.is_empty());
        assert_eq!(text, document.to_map_text());
        assert!(settings.generated_materials.is_empty());
    }

    #[test]
    fn translucent_paint_keeps_every_texel_of_a_face_one_tile_wide() {
        // A 64-unit face over a 64-texel texture: the bake's last row and column lie
        // on the far edge and once took the opposite edge's texels from the next tile.
        let brush = map_model::shapes::create_cuboid(
            &Bbox3d::new(Vec3d::fill(-1024.0), Vec3d::fill(1024.0)),
            &Bbox3d::new(Vec3d::zero(), Vec3d::fill(64.0)),
            "stone",
        )
        .expect("a cube fits the world");
        let polygon = brush.face_polygon(0).expect("the face has a polygon");
        let mut face = brush.faces()[0].clone();
        face.set_paint(Some(FaceColor::Solid {
            color: Rgb::new(0, 0, 0),
            alpha: 0,
        }));
        let texels: Vec<u8> = (0..64u8)
            .flat_map(|y| (0..64u8).flat_map(move |x| [x * 4, y * 4, 0, 255]))
            .collect();
        let base = Material::rgba("stone".into(), PathBuf::new(), 64, 64, texels);
        let image = bake(&face, &polygon, Some(&base)).expect("translucent paint bakes");
        assert_eq!((image.width, image.height), (64, 64));
        let pixels = image.pixels.as_chunks::<4>().0;
        let columns: std::collections::BTreeSet<u8> = pixels.iter().map(|p| p[0]).collect();
        let rows: std::collections::BTreeSet<u8> = pixels.iter().map(|p| p[1]).collect();
        assert_eq!(columns.len(), 64, "a texel column is lost or doubled");
        assert_eq!(rows.len(), 64, "a texel row is lost or doubled");
    }

    /// The colour of `image` at normalised `uv`, read bilinearly as a clamping sampler
    /// reads it.
    fn sample(image: &BakedFace, uv: Vec2f) -> [f32; 3] {
        let (w, h) = (image.width as f32, image.height as f32);
        let x = (uv.x() * w - 0.5).clamp(0.0, w - 1.0);
        let y = (uv.y() * h - 0.5).clamp(0.0, h - 1.0);
        let (x0, y0) = (x.floor(), y.floor());
        let (x1, y1) = ((x0 + 1.0).min(w - 1.0), (y0 + 1.0).min(h - 1.0));
        let at = |x: f32, y: f32| {
            let i = ((y as u32 * image.width + x as u32) * 4) as usize;
            [0, 1, 2].map(|c| f32::from(image.pixels[i + c]) / 255.0)
        };
        let (fx, fy) = (x - x0, y - y0);
        std::array::from_fn(|c| {
            let top = at(x0, y0)[c] * (1.0 - fx) + at(x1, y0)[c] * fx;
            let bottom = at(x0, y1)[c] * (1.0 - fx) + at(x1, y1)[c] * fx;
            top * (1.0 - fy) + bottom * fy
        })
    }

    /// Every face of a box painted with one gradient shares one small image, and the
    /// image read at each corner and between them gives the colour the editor paints
    /// there, for linear and radial gradients, fitted or turned. A gradient placed on part
    /// of a face is baked for that face instead.
    #[test]
    fn gradients_share_one_image_that_paints_what_the_editor_does() {
        use map_model::brush_face::ColorStop;
        use map_model::brush_face::GradientEnds;
        use map_model::brush_face::GradientStops;
        use map_model::brush_face::PaintOnFace;
        let brush = map_model::shapes::create_cuboid(
            &Bbox3d::new(Vec3d::fill(-1024.0), Vec3d::fill(1024.0)),
            &Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(96.0, 40.0, 64.0)),
            "stone",
        )
        .expect("a box fits the world");
        let stops = GradientStops::new(&[
            ColorStop {
                position: 0.0,
                color: Rgb::new(250, 20, 10),
            },
            ColorStop {
                position: 0.4,
                color: Rgb::new(20, 240, 30),
            },
            ColorStop {
                position: 1.0,
                color: Rgb::new(10, 30, 250),
            },
        ])
        .expect("three stops");
        for radial in [false, true] {
            let paint = FaceColor::MultiGradient {
                stops,
                radial,
                angle: 30.0,
                alpha: 255,
                ends: None,
            };
            let mut images = std::collections::HashSet::new();
            for index in 0..brush.faces().len() {
                let polygon = brush.face_polygon(index).expect("a polygon");
                let mut face = brush.faces()[index].clone();
                face.set_paint(Some(paint));
                let image = bake(&face, &polygon, None).expect("a gradient bakes");
                assert!(image.encoded.is_some(), "radial {radial}: not shared");
                images.insert(image.pixels.clone());
                let sampler = PaintOnFace::new(&face, &polygon, paint);
                let centre =
                    polygon.iter().fold(Vec3d::zero(), |a, p| a + *p) / polygon.len() as f64;
                let points = polygon
                    .iter()
                    .flat_map(|p| [*p, (*p + centre) / 2.0])
                    .chain([centre]);
                for point in points {
                    let uv = face.uv_coords(point);
                    let got = sample(&image, image.uv(uv));
                    let want = sampler.at_uv(uv);
                    assert!(
                        got.iter().zip(want).all(|(g, w)| (g - w).abs() < 0.03),
                        "radial {radial}, face {index}: {got:?} where the editor has {want:?}"
                    );
                }
            }
            assert_eq!(images.len(), 1, "radial {radial}: one image for every face");
        }

        // Placed on the middle of the face, the gradient holds its end colours beyond its
        // ends, which a shared strip read by a repeating sampler would not.
        let polygon = brush.face_polygon(0).expect("a polygon");
        let mut face = brush.faces()[0].clone();
        face.set_paint(Some(FaceColor::MultiGradient {
            stops,
            radial: false,
            angle: 0.0,
            alpha: 255,
            ends: Some(GradientEnds {
                start: [0.4, 0.5],
                end: [0.6, 0.5],
            }),
        }));
        let image = bake(&face, &polygon, None).expect("a gradient bakes");
        assert!(image.encoded.is_none(), "baked for the face");
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    #[test]
    fn image_cache_matches_uncached_pixels_and_does_not_reuse_replaced_material() {
        let world = geometry_math::bbox::Bbox3d::from_scalars(-4096.0, 4096.0);
        let brush = map_model::shapes::create_cuboid(
            &world,
            &geometry_math::bbox::Bbox3d::from_scalars(-32.0, 32.0),
            "stone",
        )
        .unwrap();
        let mut face = brush.faces()[0].clone();
        face.set_paint(Some(
            FaceColor::solid(map_model::brush_face::Rgb::new(20, 200, 80)).with_alpha(128),
        ));
        let polygon = brush.face_polygon(0).unwrap();
        let red = Material::rgba(
            "stone".into(),
            "/tmp/cache-material.png".into(),
            1,
            1,
            vec![255, 0, 0, 255],
        );
        let blue = Material::rgba(
            "stone".into(),
            "/tmp/cache-material.png".into(),
            1,
            1,
            vec![0, 0, 255, 255],
        );
        let first = prepare_image(&face, Some(&polygon), Some(&red)).unwrap();
        let repeated = prepare_image(&face, Some(&polygon), Some(&red)).unwrap();
        assert_eq!(first.png, repeated.png);
        assert_eq!(
            image::load_from_memory(&first.png)
                .unwrap()
                .to_rgba8()
                .as_raw(),
            &bake(&face, &polygon, Some(&red)).unwrap().pixels
        );
        assert_ne!(
            first.png,
            prepare_image(&face, Some(&polygon), Some(&blue))
                .unwrap()
                .png
        );
    }
}
