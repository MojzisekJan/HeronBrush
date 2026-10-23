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
    /// Affine transform from the original face UV to pixel coordinates in the bake.
    pub scale: [f32; 2],
    pub offset: [f32; 2],
}
impl BakedFace {
    pub fn uv(&self, original: Vec2f) -> Vec2f {
        Vec2f::new(
            (original.x() * self.scale[0] + self.offset[0]) / self.width as f32,
            (original.y() * self.scale[1] + self.offset[1]) / self.height as f32,
        )
    }
    pub fn png(&self) -> Result<Vec<u8>, std::io::Error> {
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
            scale: [0.0; 2],
            offset: [0.5; 2],
        });
    }
    if polygon.len() < 3 {
        return None;
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
    let sampler = map_model::brush_face::PaintOnFace::new(face, polygon, paint);
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
        scale,
        offset: [0.5 - lo[0] * scale[0], 0.5 - lo[1] * scale[1]],
    })
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
                        scale: [1.0; 2],
                        offset: [0.0; 2],
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
    scale: [f32; 2],
    offset: [f32; 2],
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
                    scale: [1.0, 1.0],
                    offset: [0.0, 0.0],
                })
            })
            .unwrap_or(BakedFace {
                width: 1,
                height: 1,
                pixels: vec![190, 190, 190, 255],
                scale: [0.0; 2],
                offset: [0.5; 2],
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
        scale: baked.scale,
        offset: baked.offset,
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
                let su = f64::from(map_model::uv::safe_scale(face.uv_attributes.scale.x()));
                let sv = f64::from(map_model::uv::safe_scale(face.uv_attributes.scale.y()));
                face.uv_coord_system = map_model::uv::UvCoordSystem::Parallel {
                    u_axis: u * (f64::from(image.scale[0]) / su),
                    v_axis: v * (f64::from(image.scale[1]) / sv),
                };
                face.uv_attributes.offset = Vec2f::new(
                    face.uv_attributes.offset.x() * image.scale[0] + image.offset[0],
                    face.uv_attributes.offset.y() * image.scale[1] + image.offset[1],
                );
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
