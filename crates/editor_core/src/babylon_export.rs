//! Portable Babylon lighting captures and camera display settings.
use super::{Contents, Error};
use crate::document::Document;
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn enhance(
    document: &Document,
    scale: f64,
    directory: &Path,
    name: &str,
    value: &mut Value,
    files: &mut [(std::path::PathBuf, Contents)],
) -> Result<(), Error> {
    let mut settings = crate::render_camera::cameras(document)
        .into_iter()
        .next()
        .map_or_else(crate::render_settings::from_preferences, |camera| {
            camera.2.render()
        });
    let env = &mut value["environment"];
    if !env.is_object() {
        return Ok(());
    }
    let grading = settings.color_grading;
    let mapper = grading.tone_map.unwrap_or_else(|| {
        crate::preferences::pref(|p| match p.tone_mapper {
            crate::preferences::ToneMapper::Aces => render::color::ToneMap::Aces,
            crate::preferences::ToneMapper::Agx => render::color::ToneMap::Agx,
            crate::preferences::ToneMapper::KhronosNeutral => {
                render::color::ToneMap::KhronosNeutral
            }
            crate::preferences::ToneMapper::TonyMcMapface => render::color::ToneMap::TonyMcMapface,
        })
    });
    env["shadows"] = json!(settings.shadows);
    env["display"] = json!({"exposure":settings.exposure,"white_balance":grading.white_balance(),"contrast":grading.contrast,"saturation":grading.saturation,"tone_map":match mapper {render::color::ToneMap::Aces=>"aces",render::color::ToneMap::Agx=>"agx",render::color::ToneMap::KhronosNeutral=>"neutral",render::color::ToneMap::TonyMcMapface=>"tony"}});
    if mapper == render::color::ToneMap::TonyMcMapface {
        env["display"]["tony_lut"] = json!(scene_export::encode_base64(include_bytes!(
            "../../render_wgpu/assets/tony_mc_mapface_48.rgb9e5"
        )));
    }
    let source = crate::environment::settings(document);
    if !source.path.as_os_str().is_empty() {
        let path = crate::environment::resolve_path(document, &source.path);
        let image = crate::environment::load(&path).map_err(Error::Map)?;
        let mut pixels = Vec::with_capacity(1024 * 512);
        for y in 0..512 {
            for x in 0..1024 {
                let direction = scene_export::writer::sky::from_export(
                    scene_export::writer::sky::babylon_panorama_direction(
                        (x as f32 + 0.5) / 1024.0,
                        (y as f32 + 0.5) / 512.0,
                    ),
                );
                pixels.push(image::Rgb(
                    image
                        .sample(direction, source.rotation_degrees)
                        .map(|v| (v * source.intensity).min(65504.0)),
                ));
            }
        }
        let mut bytes = Vec::new();
        image::codecs::hdr::HdrEncoder::new(&mut bytes)
            .encode(&pixels, 1024, 512)
            .map_err(|e| Error::Map(e.to_string()))?;
        let path = directory.join(format!("{name}/sky.hdr"));
        let item = files
            .iter_mut()
            .find(|(p, _)| p == &path)
            .ok_or_else(|| Error::Map("Babylon sky output is missing".into()))?;
        item.1 = Contents::Bytes(bytes);
        env["show_background"] = json!(source.show_background);
        settings.environment = Some(render::environment::Environment {
            image,
            rotation_degrees: source.rotation_degrees,
            intensity: source.intensity,
            show_background: source.show_background,
        });
    }
    let config = crate::light_probes::settings(document);
    if !config.probes.is_empty() {
        let mut capture = crate::render_settings::from_preferences();
        capture.environment = settings.environment.clone();
        if let Some(lighting) = &config.lighting {
            lighting.apply(&mut capture);
            capture.sun_elevation = config.sun_elevation;
            capture.sun_azimuth = config.sun_azimuth;
            capture.shadows = config.shadows;
        }
        let baked =
            crate::light_probes::bake_blocking(document, &config, &capture).map_err(Error::Map)?;
        let width = 384usize;
        let height = 127 * baked.probes().len();
        let mut atlas = vec![0u8; width * height * 4];
        let mut probes = Vec::new();
        for (i, probe) in baked.probes().iter().enumerate() {
            let mut row = i * 127;
            for level in 0..7 {
                let size = 64usize >> level;
                let source_level = level.min(probe.size.ilog2() as usize);
                let source_size = (probe.size as usize >> source_level).max(1);
                for face in 0..6 {
                    for y in 0..size {
                        for x in 0..size {
                            let pixel = probe.mips[source_level][face * source_size * source_size
                                + (y * source_size / size) * source_size
                                + x * source_size / size];
                            let offset = ((row + y) * width + face * size + x) * 4;
                            atlas[offset..offset + 4]
                                .copy_from_slice(&rgbe([pixel[0], pixel[1], pixel[2]]));
                        }
                    }
                }
                row += size;
            }
            let v = &probe.volume;
            probes.push(json!({"position":v.position,"half_size":v.half_size,"blend":v.blend,"intensity":v.intensity,"reflections":v.reflections,"diffuse":v.diffuse,"sh":probe.sh,"max_lod":probe.size.ilog2()}));
        }
        env["local_lighting"] = json!({"version":1,"scale":scale,"width":width,"height":height,"atlas":scene_export::encode_base64(&atlas),"probes":probes});
    }
    Ok(())
}
fn rgbe(c: [f32; 3]) -> [u8; 4] {
    let peak = c.into_iter().fold(0.0f32, f32::max);
    if peak < 1e-32 {
        return [0; 4];
    }
    let exponent = peak.log2().floor() as i32 + 1;
    let factor = 256.0 / 2.0f32.powi(exponent);
    [
        (c[0] * factor) as u8,
        (c[1] * factor) as u8,
        (c[2] * factor) as u8,
        (exponent + 128) as u8,
    ]
}
/// The standalone page embeds the exact same environment payload and HDR image as
/// the external level, so local files and hosted levels use one lighting path.
pub(super) fn embed_page(page: String, level: &str, panorama: Option<&[u8]>) -> String {
    let marker = "<script id=\"level-data\" type=\"application/json\">";
    let start = page.find(marker).expect("Babylon page data") + marker.len();
    let end = start
        + page[start..]
            .find("</script>")
            .expect("Babylon page data end");
    let mut data: Value = serde_json::from_str(&page[start..end]).expect("Babylon page JSON");
    data["level"] = serde_json::from_str(level).expect("level JSON");
    if let Some(bytes) = panorama {
        data["sky"] = json!(scene_export::encode_base64(bytes));
    }
    let mut result = page.clone();
    result.replace_range(start..end, &data.to_string().replace('<', "\\u003c"));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{export_files, Options, Target};
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    use map_model::brush_face::{FaceColor, Rgb, SurfaceMaterial};
    fn cube() -> Document {
        let mut doc = Document::new(Default::default());
        doc.add_brushes(vec![map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &Bbox3d::new(Vec3d::zero(), Vec3d::fill(64.0)),
            "stone",
        )
        .unwrap()]);
        doc
    }
    struct Scratch(std::path::PathBuf);
    impl Scratch {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("heron-babylon-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn options(path: &Path) -> Options {
        Options {
            target: Target::Babylon,
            name: "arena".into(),
            project_dir: path.to_string_lossy().into_owned(),
            texture_dir: "textures".into(),
            ..Default::default()
        }
    }
    fn glb(files: &[(std::path::PathBuf, Contents)]) -> (Value, Vec<u8>) {
        let bytes = files
            .iter()
            .find_map(|(p, c)| match c {
                Contents::Bytes(b) if p.extension().is_some_and(|e| e == "glb") => Some(b),
                _ => None,
            })
            .unwrap();
        let n = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(bytes[20 + n..24 + n].try_into().unwrap()) as usize;
        (
            serde_json::from_slice(&bytes[20..20 + n]).unwrap(),
            bytes[28 + n..28 + n + size].to_vec(),
        )
    }
    #[test]
    fn babylon_packs_loose_scalar_maps_and_preserves_emission() {
        let root = Scratch::new("maps");
        let textures = root.0.join("textures");
        std::fs::create_dir_all(&textures).unwrap();
        for (suffix, pixel) in [
            ("", [90, 90, 90, 255]),
            ("_roughness", [64, 64, 64, 255]),
            ("_metallic", [128, 128, 128, 255]),
            ("_ao", [192, 192, 192, 255]),
            ("_emission", [30, 80, 150, 255]),
        ] {
            std::fs::write(
                textures.join(format!("stone{suffix}.png")),
                crate::assets::encode_png(2, 2, &pixel.repeat(4)).unwrap(),
            )
            .unwrap();
        }
        let mut document = cube();
        document.load_materials(&textures).unwrap();
        let (_, files) = export_files(&document, &root.0, &options(&root.0)).unwrap();
        let (doc, bin) = glb(&files);
        let mat = &doc["materials"][0];
        assert!(mat["occlusionTexture"].is_object());
        assert!(mat["emissiveTexture"].is_object());
        assert_eq!(mat["emissiveFactor"], json!([1.0, 1.0, 1.0]));
        let texture = mat["pbrMetallicRoughness"]["metallicRoughnessTexture"]["index"]
            .as_u64()
            .unwrap() as usize;
        let image = doc["textures"][texture]["source"].as_u64().unwrap() as usize;
        let view = doc["images"][image]["bufferView"].as_u64().unwrap() as usize;
        let offset = doc["bufferViews"][view]["byteOffset"].as_u64().unwrap() as usize;
        let size = doc["bufferViews"][view]["byteLength"].as_u64().unwrap() as usize;
        let pixels = image::load_from_memory(&bin[offset..offset + size])
            .unwrap()
            .to_rgba8();
        assert_eq!(pixels.get_pixel(0, 0).0, [255, 64, 128, 255]);
    }
    #[test]
    fn babylon_preserves_layered_face_materials_and_tangents() {
        let root = Scratch::new("layers");
        let mut document = cube();
        document.select_all();
        document.set_face_color(Some(FaceColor::solid(Rgb::new(220, 100, 40))));
        document.set_surface_material(Some(SurfaceMaterial {
            clearcoat: 80,
            coat_roughness: 20,
            anisotropy: 70,
            anisotropy_rotation: 45,
            emission: 300,
            ..Default::default()
        }));
        let (_, files) = export_files(&document, &root.0, &options(&root.0)).unwrap();
        let (doc, _) = glb(&files);
        let mat = &doc["materials"][0];
        let e = &mat["extensions"];
        assert!(
            (e["KHR_materials_clearcoat"]["clearcoatFactor"]
                .as_f64()
                .unwrap()
                - 0.8)
                .abs()
                < 1e-5
        );
        assert!(
            (e["KHR_materials_anisotropy"]["anisotropyRotation"]
                .as_f64()
                .unwrap()
                - std::f64::consts::FRAC_PI_4)
                .abs()
                < 1e-5
        );
        assert_eq!(
            e["KHR_materials_emissive_strength"]["emissiveStrength"],
            json!(3.0)
        );
        assert_eq!(doc["extensionsUsed"].as_array().unwrap().len(), 3);
        for p in doc["meshes"][0]["primitives"].as_array().unwrap() {
            assert!(p["attributes"]["TANGENT"].is_number());
        }
    }
    #[test]
    fn babylon_embeds_hdr_grading_and_baked_volumes_in_both_entry_points() {
        let root = Scratch::new("lighting");
        let mut document = cube();
        let path = root.0.join("environment.hdr");
        let mut hdr = Vec::new();
        image::codecs::hdr::HdrEncoder::new(&mut hdr)
            .encode(&vec![image::Rgb([2.0, 0.5, 0.25]); 8], 4, 2)
            .unwrap();
        std::fs::write(&path, hdr).unwrap();
        crate::environment::save(
            &mut document,
            &crate::environment::Settings {
                path,
                rotation_degrees: 90.0,
                intensity: 2.0,
                show_background: false,
            },
        );
        let id =
            document.create_point_entity(crate::render_camera::CLASS, Vec3d::new(0.0, 0.0, 100.0));
        let camera = crate::render_camera::Settings {
            exposure: 1.5,
            imaging: crate::render_camera::Imaging {
                exposure_ev: 1.0,
                temperature: 8000.0,
                tint: 0.2,
                contrast: 1.3,
                saturation: 0.7,
                tone_map: Some(crate::preferences::ToneMapper::TonyMcMapface),
            },
            ..Default::default()
        };
        document.set_entity_property_on(
            &[id],
            crate::render_camera::KEY,
            &serde_json::to_string(&camera).unwrap(),
        );
        crate::light_probes::save(
            &mut document,
            &crate::light_probes::Settings {
                resolution: 16,
                probes: vec![crate::light_probes::Probe::default()],
                ..Default::default()
            },
        );
        let (_, files) = export_files(&document, &root.0, &options(&root.0)).unwrap();
        let text = |ext: &str| {
            files
                .iter()
                .find_map(|(p, c)| match c {
                    Contents::Text(s) if p.extension().is_some_and(|e| e == ext) => Some(s),
                    _ => None,
                })
                .unwrap()
        };
        let level: Value = serde_json::from_str(text("json")).unwrap();
        let marker = "<script id=\"level-data\" type=\"application/json\">";
        let data = text("html")
            .split(marker)
            .nth(1)
            .unwrap()
            .split("</script>")
            .next()
            .unwrap();
        let data: Value = serde_json::from_str(data).unwrap();
        assert_eq!(level["environment"], data["level"]["environment"]);
        let env = &level["environment"];
        assert_eq!(env["show_background"], false);
        assert_eq!(env["display"]["exposure"], 3.0);
        assert_eq!(env["display"]["tone_map"], "tony");
        assert!(env["display"]["tony_lut"].as_str().unwrap().len() > 1000);
        assert_eq!(env["local_lighting"]["probes"].as_array().unwrap().len(), 1);
        assert_eq!(env["local_lighting"]["scale"], 32.0);
        let hdr = files
            .iter()
            .find_map(|(p, c)| match c {
                Contents::Bytes(b) if p.extension().is_some_and(|e| e == "hdr") => Some(b),
                _ => None,
            })
            .unwrap();
        let pixel = image::load_from_memory_with_format(hdr, image::ImageFormat::Hdr)
            .unwrap()
            .to_rgb32f()
            .get_pixel(0, 0)
            .0;
        assert!((pixel[0] - 4.0).abs() < 0.05);
        assert!((pixel[1] - 1.0).abs() < 0.05);
        assert_eq!(data["sky"], scene_export::encode_base64(hdr));
    }
}
