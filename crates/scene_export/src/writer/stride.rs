//! Stride 4.4 export: native model, material and texture assets beside the GLBs, plus
//! generated C# that builds the level, adds Bullet colliders and plays animation.
//!
//! Stride compiles assets, not files. A `.glb` dropped into a project is nothing until a
//! `.sdm3d` names it as a model source, and a model draws with the default material until
//! its `Materials` list points at `.sdmat` assets. So every mesh gets a model asset, every
//! glTF material a material asset, and every material image a texture asset, all with
//! ids derived from their content URL so a re-export keeps the references stable.
//!
//! The images come out of the GLBs themselves rather than from wherever the editor found
//! them: an archive texture has no file to point at, and the GLB already holds exactly the
//! pixels the level draws with.
use crate::level::{CollisionMesh, Level, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub struct StrideOptions {
    /// The content URL of the folder the meshes land in: the path of `<export>/<level>`
    /// below the package's `Assets` folder, without a leading slash.
    pub content_dir: String,
    /// The sky and sun to write as a native skybox and directional light, if any.
    pub sky: Option<super::sky::Sky>,
}
impl StrideOptions {
    pub fn for_level(name: &str) -> Self {
        Self {
            content_dir: format!("Levels/{name}"),
            sky: None,
        }
    }
}

/// The side of the exported sky cube's faces: a pixel is 0.7 degrees, which keeps the
/// sun's glow round.
const SKY_SIZE: u32 = 128;

/// A mesh the exporter wrote, by its file name in `<export>/<level>/` and its bytes.
pub struct StrideMesh<'a> {
    pub file_name: &'a str,
    pub glb: &'a [u8],
}

pub struct StrideFile {
    /// Relative to the export directory.
    pub path: String,
    pub contents: Vec<u8>,
}

pub fn validate(level: &Level) -> Result<(), String> {
    for entity in &level.entities {
        for key in ["stride_model", "stride_prefab"] {
            if let Some(value) = entity.properties.get(key) {
                if !matches!(value,Value::String(url) if !url.trim().is_empty()&&!url.starts_with('/')&&!url.contains('\\')&&!url.split('/').any(|part|part==".."))
                {
                    return Err(format!(
                        "{}: {key} must be a project content URL",
                        entity.name
                    ));
                }
            }
        }
    }
    Ok(())
}

fn literal(text: &str) -> String {
    serde_json::to_string(text)
        .expect("string literal")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}
fn hint<'a>(entity: &'a crate::level::Entity, key: &str) -> Option<&'a str> {
    match entity.properties.get(key) {
        Some(Value::String(url)) => Some(url),
        _ => None,
    }
}
fn fnv(text: &str, seed: u64) -> u64 {
    let mut hash = seed;
    for byte in text.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    }
    hash
}

/// A stable asset id for a content URL, formatted the way Stride writes them.
pub fn asset_id(url: &str) -> String {
    let a = fnv(url, 0xcbf29ce484222325);
    let b = fnv(url, 0x84222325cbf29ce4);
    let hex = format!("{a:016X}{b:016X}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}
/// The 32-hex item key Stride puts in front of every collection entry.
fn item_key(text: &str) -> String {
    format!(
        "{:016x}{:016x}",
        fnv(text, 0x9e3779b97f4a7c15),
        fnv(text, 0xc2b2ae3d27d4eb4f)
    )
}
/// A file-name-safe version of a material name, which can hold `/` or `*`.
fn asset_name(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if safe == name {
        safe
    } else {
        format!("{safe}_{:08x}", fnv(name, 0xcbf29ce484222325) as u32)
    }
}

#[derive(Debug)]
struct GlbImage {
    bytes: Vec<u8>,
    extension: &'static str,
    clamp_u: bool,
    clamp_v: bool,
}

#[derive(Debug)]
struct GlbMaterial {
    name: String,
    base_color: [f64; 4],
    metallic: f64,
    roughness: f64,
    double_sided: bool,
    blend: bool,
    image: Option<GlbImage>,
    normal: Option<GlbImage>,
    normal_scale: f64,
    metallic_roughness: Option<GlbImage>,
    occlusion: Option<GlbImage>,
    occlusion_strength: f64,
    emissive: [f64; 3],
    emissive_strength: f64,
    emissive_image: Option<GlbImage>,
    alpha_cutoff: Option<f64>,
}

impl GlbMaterial {
    /// Names in different GLBs are not globally unique. Include pixels, factors and
    /// samplers when deciding whether two named materials can share a native asset.
    fn fingerprint(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        let mut feed = |bytes: &[u8]| {
            for &byte in bytes {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
        };
        for value in self
            .base_color
            .into_iter()
            .chain([
                self.metallic,
                self.roughness,
                self.normal_scale,
                self.emissive_strength,
                self.occlusion_strength,
            ])
            .chain(self.emissive)
        {
            feed(&value.to_bits().to_le_bytes());
        }
        feed(&[
            u8::from(self.double_sided),
            u8::from(self.blend),
            u8::from(self.alpha_cutoff.is_some()),
        ]);
        feed(&self.alpha_cutoff.unwrap_or(0.).to_bits().to_le_bytes());
        for image in [
            &self.image,
            &self.normal,
            &self.emissive_image,
            &self.metallic_roughness,
            &self.occlusion,
        ] {
            feed(&[u8::from(image.is_some())]);
            if let Some(image) = image {
                feed(&(image.bytes.len() as u64).to_le_bytes());
                feed(&image.bytes);
                feed(image.extension.as_bytes());
                feed(&[u8::from(image.clamp_u), u8::from(image.clamp_v)]);
            }
        }
        hash
    }
}

fn glb_materials(glb: &[u8]) -> Vec<GlbMaterial> {
    let Some(json_length) = glb
        .get(12..16)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
    else {
        return Vec::new();
    };
    let Some(doc) = glb
        .get(20..20 + json_length)
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
    else {
        return Vec::new();
    };
    let bin = glb.get(20 + json_length..).and_then(|rest| {
        let length = u32::from_le_bytes(rest.get(0..4)?.try_into().ok()?) as usize;
        rest.get(8..8 + length)
    });
    let image = |texture: &serde_json::Value| -> Option<GlbImage> {
        let texture = &doc["textures"][texture["index"].as_u64()? as usize];
        let sampler = texture["sampler"]
            .as_u64()
            .map(|s| &doc["samplers"][s as usize]);
        let clamp_u = sampler.is_some_and(|s| s["wrapS"] == 33071);
        let clamp_v = sampler.is_some_and(|s| s["wrapT"] == 33071);
        let image = &doc["images"][texture["source"].as_u64()? as usize];
        let view = &doc["bufferViews"][image["bufferView"].as_u64()? as usize];
        let start = view["byteOffset"].as_u64().unwrap_or(0) as usize;
        let bytes = bin?
            .get(start..start + view["byteLength"].as_u64()? as usize)?
            .to_vec();
        let extension = if image["mimeType"] == "image/jpeg" {
            "jpg"
        } else {
            "png"
        };
        Some(GlbImage {
            bytes,
            extension,
            clamp_u,
            clamp_v,
        })
    };
    doc["materials"]
        .as_array()
        .map(|materials| {
            materials
                .iter()
                .enumerate()
                .map(|(index, m)| {
                    let pbr = &m["pbrMetallicRoughness"];
                    let base = pbr["baseColorFactor"]
                        .as_array()
                        .map(|v| {
                            [0, 1, 2, 3].map(|i| v.get(i).and_then(|x| x.as_f64()).unwrap_or(1.))
                        })
                        .unwrap_or([1.; 4]);
                    let found = image(&pbr["baseColorTexture"]);
                    GlbMaterial {
                        name: m["name"]
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("material_{index}")),
                        base_color: base,
                        metallic: pbr["metallicFactor"].as_f64().unwrap_or(1.),
                        roughness: pbr["roughnessFactor"].as_f64().unwrap_or(1.),
                        double_sided: m["doubleSided"].as_bool().unwrap_or(false),
                        blend: m["alphaMode"] == "BLEND",
                        image: found,
                        normal: image(&m["normalTexture"]),
                        metallic_roughness: image(&pbr["metallicRoughnessTexture"]),
                        occlusion: image(&m["occlusionTexture"]),
                        occlusion_strength: m["occlusionTexture"]["strength"]
                            .as_f64()
                            .unwrap_or(1.),
                        normal_scale: m["normalTexture"]["scale"].as_f64().unwrap_or(1.),
                        emissive: [0, 1, 2].map(|i| m["emissiveFactor"][i].as_f64().unwrap_or(0.)),
                        emissive_strength:
                            m["extensions"]["KHR_materials_emissive_strength"]["emissiveStrength"]
                                .as_f64()
                                .unwrap_or(1.),
                        emissive_image: image(&m["emissiveTexture"]),
                        alpha_cutoff: (m["alphaMode"] == "MASK")
                            .then(|| m["alphaCutoff"].as_f64().unwrap_or(0.5)),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Width and height from a PNG's header.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(..8)? != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    Some((
        u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?),
        u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?),
    ))
}

/// A colour map multiplied by a linear factor. Every slot keeps its own sampler.
fn color_map(
    text: &mut String,
    key: &str,
    factor: [f64; 4],
    image: Option<&GlbImage>,
    texture: Option<&str>,
) {
    let [r, g, b, a] = factor;
    let tint = factor != [1.; 4];
    if let Some((image, texture)) = image.zip(texture) {
        let indent = if tint {
            "                "
        } else {
            "            "
        };
        if tint {
            let _ = write!(
                text,
                "        {key}: !ComputeBinaryColor\n            Operator: Multiply\n            LeftChild: !ComputeTextureColor\n"
            );
        } else {
            let _ = write!(text, "        {key}: !ComputeTextureColor\n");
        }
        let u = if image.clamp_u { "Clamp" } else { "Wrap" };
        let v = if image.clamp_v { "Clamp" } else { "Wrap" };
        let _ = write!(
            text,
            "{indent}Texture: {texture}\n{indent}FallbackValue:\n{indent}    Value: {{R: 1.0, G: 1.0, B: 1.0, A: 1.0}}\n{indent}AddressModeU: {u}\n{indent}AddressModeV: {v}\n{indent}Scale: {{X: 1.0, Y: 1.0}}\n{indent}Offset: {{X: 0.0, Y: 0.0}}\n{indent}Swizzle: null\n"
        );
        if tint {
            let _ = write!(
                text,
                "            RightChild: !ComputeColor\n                Value: {{R: {r}, G: {g}, B: {b}, A: {a}}}\n"
            );
        }
    } else {
        let _ = write!(
            text,
            "        {key}: !ComputeColor\n            Value: {{R: {r}, G: {g}, B: {b}, A: {a}}}\n"
        );
    }
}

/// A scalar map with glTF's factor, optionally mixed with white for AO strength.
fn scalar_map(
    text: &mut String,
    key: &str,
    image: &GlbImage,
    texture: &str,
    channel: &str,
    factor: f64,
    offset: f64,
) {
    let u = if image.clamp_u { "Clamp" } else { "Wrap" };
    let v = if image.clamp_v { "Clamp" } else { "Wrap" };
    let mut node = format!(
        "!ComputeTextureScalar\n    Texture: {texture}\n    Channel: {channel}\n    AddressModeU: {u}\n    AddressModeV: {v}\n    FallbackValue: !ComputeFloat\n        Value: 1.0\n"
    );
    for (op, value, enabled) in [
        ("Multiply", factor, factor != 1.),
        ("Add", offset, offset != 0.),
    ] {
        if enabled {
            let child = node
                .lines()
                .skip(1)
                .map(|l| format!("    {l}\n"))
                .collect::<String>();
            node = format!(
                "!ComputeBinaryScalar\n    Operator: {op}\n    LeftChild: {}\n{child}    RightChild: !ComputeFloat\n        Value: {value}\n",
                node.lines().next().unwrap()
            );
        }
    }
    let _ = write!(text, "        {key}: {}\n", node.lines().next().unwrap());
    for line in node.lines().skip(1) {
        let _ = writeln!(text, "        {line}");
    }
}

fn material_asset(
    id: &str,
    material: &GlbMaterial,
    texture: Option<&str>,
    normal: Option<&str>,
    emissive: Option<&str>,
    metallic_roughness: Option<&str>,
    occlusion: Option<&str>,
) -> String {
    let mut text = format!(
        "!MaterialAsset\nId: {id}\nSerializedVersion: {{Stride: 2.0.0.0}}\nTags: []\nAttributes:\n"
    );
    text.push_str("    MicroSurface: !MaterialGlossinessMapFeature\n");
    if let Some((image, texture)) = material.metallic_roughness.as_ref().zip(metallic_roughness) {
        text.push_str("        Invert: true\n");
        scalar_map(
            &mut text,
            "GlossinessMap",
            image,
            texture,
            "G",
            material.roughness.clamp(0., 1.),
            0.,
        );
    } else {
        let _ = write!(
            text,
            "        GlossinessMap: !ComputeFloat\n            Value: {}\n",
            1. - material.roughness.clamp(0., 1.)
        );
    }
    text.push_str("    Diffuse: !MaterialDiffuseMapFeature\n");
    color_map(
        &mut text,
        "DiffuseMap",
        material.base_color,
        material.image.as_ref(),
        texture,
    );
    if normal.is_some() {
        text.push_str("    Surface: !MaterialNormalMapFeature\n        ScaleAndBias: true\n        IsXYNormal: false\n");
        // glTF scales signed XY before normalization. Apply the equivalent affine
        // operation to unsigned texture values before Stride unpacks them.
        let scale = material.normal_scale;
        let mut map = String::new();
        color_map(
            &mut map,
            "NormalMap",
            [scale, scale, 1., 1.],
            material.normal.as_ref(),
            normal,
        );
        if scale == 1. {
            text.push_str(&map);
        } else {
            text.push_str("        NormalMap: !ComputeBinaryColor\n            Operator: Add\n");
            for (i, line) in map.lines().enumerate() {
                if i == 0 {
                    text.push_str(&line.replacen("NormalMap:", "    LeftChild:", 1));
                } else {
                    text.push_str("    ");
                    text.push_str(line);
                }
                text.push('\n');
            }
            let offset = (1. - scale) * 0.5;
            let _ = write!(
                text,
                "            RightChild: !ComputeColor\n                Value: {{R: {offset}, G: {offset}, B: 0.0, A: 0.0}}\n"
            );
        }
    }
    if material.emissive.iter().any(|v| *v > 0.) {
        text.push_str("    Emissive: !MaterialEmissiveMapFeature\n");
        let [r, g, b] = material.emissive;
        color_map(
            &mut text,
            "EmissiveMap",
            [r, g, b, 1.],
            material.emissive_image.as_ref(),
            emissive,
        );
        let _ = write!(
            text,
            "        Intensity: !ComputeFloat\n            Value: {}\n        UseAlpha: false\n",
            material.emissive_strength
        );
    }
    text.push_str("    DiffuseModel: !MaterialDiffuseLambertModelFeature {}\n");
    text.push_str("    Specular: !MaterialMetalnessMapFeature\n");
    if let Some((image, texture)) = material.metallic_roughness.as_ref().zip(metallic_roughness) {
        scalar_map(
            &mut text,
            "MetalnessMap",
            image,
            texture,
            "B",
            material.metallic.clamp(0., 1.),
            0.,
        );
    } else {
        let _ = write!(
            text,
            "        MetalnessMap: !ComputeFloat\n            Value: {}\n",
            material.metallic.clamp(0., 1.)
        );
    }
    if let Some((image, texture)) = material.occlusion.as_ref().zip(occlusion) {
        text.push_str("    Occlusion: !MaterialOcclusionMapFeature\n        CavityMap: null\n");
        let strength = material.occlusion_strength.clamp(0., 1.);
        scalar_map(
            &mut text,
            "AmbientOcclusionMap",
            image,
            texture,
            "R",
            strength,
            1. - strength,
        );
    }
    text.push_str("    SpecularModel: !MaterialSpecularMicrofacetModelFeature\n        Fresnel: !MaterialSpecularMicrofacetFresnelSchlick {}\n        Visibility: !MaterialSpecularMicrofacetVisibilitySmithSchlickGGX {}\n        NormalDistribution: !MaterialSpecularMicrofacetNormalDistributionGGX {}\n        Environment: !MaterialSpecularMicrofacetEnvironmentGGXLUT {}\n");
    if material.blend {
        // See-through by the diffuse colour's alpha, as glTF's BLEND is.
        text.push_str("    Transparency: !MaterialTransparencyBlendFeature\n        Alpha: !ComputeFloat\n            Value: 1.0\n        Tint: !ComputeColor\n            Value: {R: 1.0, G: 1.0, B: 1.0, A: 1.0}\n");
    }
    if let Some(cutoff) = material.alpha_cutoff {
        let _ = write!(
            text,
            "    Transparency: !MaterialTransparencyCutoffFeature\n        Alpha: !ComputeFloat\n            Value: {}\n",
            cutoff.clamp(0., 1.)
        );
    }
    if material.double_sided {
        text.push_str("    CullMode: None\n");
    }
    text.push_str("    Overrides:\n        UVScale: {X: 1.0, Y: 1.0}\nLayers: {}\n");
    text
}

/// The root assets a re-export owns, so the package can be updated without touching any
/// other root asset: every `<id>:<url>` the level loads by URL at runtime.
pub fn root_assets(files: &[StrideFile], options: &StrideOptions) -> Vec<String> {
    let prefix = options.content_dir.trim_matches('/');
    let models = files
        .iter()
        .filter(|f| f.path.ends_with(".sdm3d"))
        .map(|f| {
            f.path
                .rsplit('/')
                .next()
                .unwrap()
                .trim_end_matches(".sdm3d")
                .to_string()
        });
    let sky = files
        .iter()
        .any(|f| f.path.ends_with("/Sky/Skybox.sdsky"))
        .then(|| ["Sky/Skybox".to_string(), "Sky/Sky".to_string()])
        .into_iter()
        .flatten();
    models
        .chain(sky)
        .map(|stem| {
            let url = format!("{prefix}/{stem}");
            format!("{}:{url}", asset_id(&url))
        })
        .collect()
}

/// Rewrites a package's `RootAssets` so it lists `roots` and no other entry under the
/// level's content folder. Entries outside it, and the rest of the file, stay as they are.
pub fn update_package(package: &str, roots: &[String], content_dir: &str) -> String {
    let owned = format!(":{}/", content_dir.trim_matches('/'));
    let mut out = Vec::new();
    let mut kept = Vec::new();
    let mut lines = package.lines().peekable();
    let mut found = false;
    while let Some(line) = lines.next() {
        if line == "RootAssets:" || line == "RootAssets: []" {
            found = true;
            if line == "RootAssets:" {
                while let Some(next) = lines.peek() {
                    if next.starts_with(' ') || next.starts_with('-') {
                        let entry = lines.next().unwrap();
                        if !entry.contains(&owned) {
                            kept.push(entry.to_string());
                        }
                    } else {
                        break;
                    }
                }
            }
            out.push("RootAssets:".to_string());
            out.append(&mut kept);
            out.extend(roots.iter().map(|root| format!("    - {root}")));
        } else {
            out.push(line.to_string());
        }
    }
    if !found {
        out.push("RootAssets:".to_string());
        out.extend(roots.iter().map(|root| format!("    - {root}")));
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

pub fn write(
    level: &Level,
    collision: &[CollisionMesh],
    options: &StrideOptions,
    meshes: &[StrideMesh],
) -> Vec<StrideFile> {
    let manifest = super::three::write(level, collision);
    let data: serde_json::Value = serde_json::from_str(&manifest).expect("level JSON");
    let content = options.content_dir.trim_matches('/').to_string();
    let mut files = Vec::new();

    // Materials are shared between the meshes of a level, so each is written once.
    let mut materials: BTreeMap<String, String> = BTreeMap::new();
    let parsed: Vec<_> = meshes
        .iter()
        .map(|mesh| (mesh, glb_materials(mesh.glb)))
        .collect();
    let mut variants: BTreeMap<String, std::collections::BTreeSet<u64>> = BTreeMap::new();
    for (_, materials) in &parsed {
        for material in materials {
            variants
                .entry(material.name.clone())
                .or_default()
                .insert(material.fingerprint());
        }
    }
    for (mesh, parsed_materials) in &parsed {
        let stem = mesh.file_name.trim_end_matches(".glb");
        let mut list = String::new();
        for (material_index, material) in parsed_materials.iter().enumerate() {
            // A see-through copy of a paint (water's surface) is a material of its own,
            // one per colour, beside the paint's opaque one.
            let name = if variants[&material.name].len() > 1 {
                asset_name(&format!(
                    "{}_pbr_{:016x}",
                    material.name,
                    material.fingerprint()
                ))
            } else if material.blend {
                asset_name(&format!(
                    "{}_blend_{:08x}",
                    material.name,
                    fnv(&format!("{:?}", material.base_color), 0xcbf29ce484222325) as u32
                ))
            } else {
                asset_name(&material.name)
            };
            let url = format!("{content}/Materials/{name}");
            let id = asset_id(&url);
            if !materials.contains_key(&name) {
                let mut write_texture = |image: &GlbImage, suffix: &str, kind: &str| {
                    let texture_name = format!("{name}{suffix}");
                    let texture_url = format!("{content}/Textures/{texture_name}");
                    let texture_id = asset_id(&texture_url);
                    let extension = image.extension;
                    files.push(StrideFile {
                        path: format!("{}/Textures/{texture_name}.{extension}", level.name),
                        contents: image.bytes.clone(),
                    });
                    let kind = if kind == "normal" {
                        "Type: !NormalMapTextureType\n    InvertY: true\n"
                    } else if kind == "linear" {
                        "Type: !ColorTextureType\n    UseSRgbSampling: false\n    PremultiplyAlpha: false\n"
                    } else {
                        "Type: !ColorTextureType\n    PremultiplyAlpha: false\n    ColorKeyColor: {R: 255, G: 0, B: 255, A: 255}\n"
                    };
                    files.push(StrideFile {
                        path: format!("{}/Textures/{texture_name}.sdtex", level.name),
                        contents: format!("!Texture\nId: {texture_id}\nSerializedVersion: {{Stride: 2.0.0.0}}\nTags: []\nSource: !file {texture_name}.{extension}\n{}{kind}",
                            if png_size(&image.bytes).is_some_and(|(w, h)| w.min(h) < 4) { "IsCompressed: false\nGenerateMipmaps: false\n" } else if kind.starts_with("Type: !NormalMap") { "IsCompressed: false\n" } else { "" }).into_bytes(),
                    });
                    format!("{texture_id}:{texture_url}")
                };
                let texture = material
                    .image
                    .as_ref()
                    .map(|i| write_texture(i, "", "color"));
                let normal = material
                    .normal
                    .as_ref()
                    .map(|i| write_texture(i, ".normal", "normal"));
                let emissive = material
                    .emissive_image
                    .as_ref()
                    .map(|i| write_texture(i, ".emissive", "color"));
                let metallic_roughness = material
                    .metallic_roughness
                    .as_ref()
                    .map(|i| write_texture(i, ".mr", "linear"));
                let occlusion = material
                    .occlusion
                    .as_ref()
                    .map(|i| write_texture(i, ".ao", "linear"));
                files.push(StrideFile {
                    path: format!("{}/Materials/{name}.sdmat", level.name),
                    contents: material_asset(
                        &id,
                        material,
                        texture.as_deref(),
                        normal.as_deref(),
                        emissive.as_deref(),
                        metallic_roughness.as_deref(),
                        occlusion.as_deref(),
                    )
                    .into_bytes(),
                });
                materials.insert(name.clone(), id.clone());
            }
            let _ = write!(
                list,
                "    {}:\n        Name: {}\n        MaterialInstance:\n            Material: {id}:{url}\n",
                item_key(&format!("{stem}/{material_index}")),
                literal(&material.name)
            );
        }
        let url = format!("{content}/{stem}");
        let materials = if list.is_empty() {
            " {}\n".to_string()
        } else {
            format!("\n{list}")
        };
        files.push(StrideFile {
            path: format!("{}/{stem}.sdm3d", level.name),
            contents: format!("!Model\nId: {}\nSerializedVersion: {{Stride: 2.0.0.0}}\nTags: []\nSource: !file {stem}.glb\nPivotPosition: {{X: 0.0, Y: 0.0, Z: 0.0}}\nMaterials:{materials}Skeleton: null\n", asset_id(&url)).into_bytes(),
        });
    }

    let mut entries = String::new();
    for (index, entity) in level.entities.iter().enumerate() {
        let model = hint(entity, "stride_model").map(str::to_owned).or_else(|| {
            entity
                .mesh
                .as_deref()
                .filter(|_| !entity.surfaces.is_empty())
                .map(|path| {
                    format!(
                        "{content}/{}",
                        path.rsplit('/')
                            .next()
                            .unwrap_or(path)
                            .trim_end_matches(".glb")
                    )
                })
        });
        let q = glam::DQuat::from_euler(
            glam::EulerRot::YXZ,
            entity.transform.rotation_yxz[1],
            entity.transform.rotation_yxz[0],
            entity.transform.rotation_yxz[2],
        );
        let vector = |values: [f64; 3]| {
            format!(
                "new Vector3({:.9}f,{:.9}f,{:.9}f)",
                values[0], values[1], values[2]
            )
        };
        entries.push_str(&format!(
            "            new Record({}, {}, {}, {}, new Quaternion({:.9}f,{:.9}f,{:.9}f,{:.9}f), {}, {}),\n",
            literal(&entity.name),
            model.as_deref().map(literal).unwrap_or("null".into()),
            hint(entity, "stride_prefab").map(literal).unwrap_or("null".into()),
            vector(entity.transform.translation),
            q.x, q.y, q.z, q.w,
            vector(entity.transform.scale),
            literal(&data["entities"][index].to_string())
        ));
    }
    let mut environment = String::from(
        "        public const string SkyboxUrl = null, SkyTextureUrl = null;\n        public static readonly Color3 SunColor = new Color3(1f,1f,1f);\n        public const float SunIntensity = 0f, AmbientIntensity = 0f;\n        public static readonly Vector3 ToSun = Vector3.UnitY;\n",
    );
    if let Some(sky) = &options.sky {
        let (texture_url, skybox_url) = (
            format!("{content}/Sky/Sky"),
            format!("{content}/Sky/Skybox"),
        );
        let texture_id = asset_id(&texture_url);
        // Stride samples a cube with the direction as it is, in its right-handed frame,
        // where Direct3D's cube layout is left-handed: the faces are written mirrored in Z.
        files.push(StrideFile {
            path: format!("{}/Sky/Sky.dds", level.name),
            contents: sky.cube_dds(SKY_SIZE, true),
        });
        files.push(StrideFile { path: format!("{}/Sky/Sky.sdtex", level.name), contents: format!("!Texture\nId: {texture_id}\nSerializedVersion: {{Stride: 2.0.0.0}}\nTags: []\nSource: !file Sky.dds\nIsCompressed: false\nType: !ColorTextureType\n    UseSRgbSampling: false\n    ColorKeyColor: {{R: 255, G: 0, B: 255, A: 255}}\n").into_bytes() });
        files.push(StrideFile { path: format!("{}/Sky/Skybox.sdsky", level.name), contents: format!("!SkyboxAsset\nId: {}\nSerializedVersion: {{Stride: 2.0.0.0}}\nTags: []\nCubeMap: {texture_id}:{texture_url}\n", asset_id(&skybox_url)).into_bytes() });
        let [r, g, b] = sky.sun_color;
        let [x, y, z] = sky.to_sun();
        environment = format!(
            "        public const string SkyboxUrl = {}, SkyTextureUrl = {};\n        public static readonly Color3 SunColor = new Color3({r:.6}f,{g:.6}f,{b:.6}f);\n        public const float SunIntensity = {:.6}f, AmbientIntensity = {:.6}f;\n        public static readonly Vector3 ToSun = new Vector3({x:.9}f,{y:.9}f,{z:.9}f);\n",
            literal(&skybox_url),
            literal(&texture_url),
            sky.sun_intensity,
            sky.ambient_intensity
        );
    }
    let namespace = format!(
        "HeronBrush.Stride.Level_{:016x}",
        fnv(
            &format!("{}\n{}", options.content_dir, level.name),
            0xcbf29ce484222325
        )
    );
    let name = literal(&level.name);
    let fill = |source: &str| {
        let mut script = String::new();
        let mut source = source;
        let replacements = [
            ("__NAMESPACE__", namespace.as_str()),
            ("__LEVEL__", name.as_str()),
            ("__ENTRIES__", entries.as_str()),
            ("__ENVIRONMENT__", environment.as_str()),
        ];
        while let Some((index, key, value)) = replacements
            .iter()
            .filter_map(|(key, value)| source.find(key).map(|index| (index, *key, *value)))
            .min_by_key(|(index, _, _)| *index)
        {
            script.push_str(&source[..index]);
            script.push_str(value);
            source = &source[index + key.len()..];
        }
        script.push_str(source);
        script.into_bytes()
    };
    files.push(StrideFile {
        path: format!("{}/HeronBrushStrideLevel.cs", level.name),
        contents: fill(include_str!("stride_importer.cs")),
    });
    files.push(StrideFile {
        path: format!("{}/HeronBrushStrideRuntime.cs", level.name),
        contents: fill(include_str!("stride_runtime.cs")),
    });
    files.push(StrideFile {
        path: format!("{}/HeronBrushStridePhysics.cs", level.name),
        contents: fill(include_str!("stride_physics.cs")),
    });
    files.push(StrideFile {
        path: format!("{}.json", level.name),
        contents: manifest.into_bytes(),
    });
    files.push(StrideFile {
        path: "STRIDE-README.md".into(),
        contents: include_str!("stride_readme.md").into(),
    });
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(files: &[StrideFile], suffix: &str) -> String {
        String::from_utf8(
            files
                .iter()
                .find(|f| f.path.ends_with(suffix))
                .unwrap_or_else(|| panic!("no {suffix}"))
                .contents
                .clone(),
        )
        .unwrap()
    }

    #[test]
    fn data_only_entities_keep_properties_and_native_placement_without_visual_models() {
        let mut entity = crate::level::Entity {
            name: "beacon".into(),
            classname: "puzzle_beacon".into(),
            ..Default::default()
        };
        entity.transform.translation = [1., 2., 3.];
        entity.transform.rotation_yxz = [0.37, -1.12, 0.64];
        entity.transform.scale = [2., 0.7, 3.1];
        entity.properties.insert("charge".into(), Value::Int(7));
        entity.properties.insert(
            "stride_prefab".into(),
            Value::String("Prefabs/Beacon".into()),
        );
        entity.targets.push("later".into());
        let level = Level {
            format: 2,
            generator: "test".into(),
            name: "workshop".into(),
            scale: 32.,
            textures: vec![],
            entities: vec![entity],
        };
        validate(&level).unwrap();
        let files = write(&level, &[], &StrideOptions::for_level("workshop"), &[]);
        let importer = text(&files, "HeronBrushStrideLevel.cs");
        assert!(importer.contains("\"beacon\", null, \"Prefabs/Beacon\""));
        assert!(importer.contains("new Vector3(1.000000000f,2.000000000f,3.000000000f)"));
        let json: serde_json::Value = serde_json::from_str(&text(&files, "workshop.json")).unwrap();
        assert_eq!(json["entities"][0]["properties"]["charge"]["value"], 7);
        assert_eq!(json["entities"][0]["targets"][0], "later");
        assert!(
            !files.iter().any(|f| f.path.ends_with(".sdm3d")),
            "no model asset without a mesh"
        );
    }

    #[test]
    fn project_content_hints_reject_invalid_types_and_parent_paths() {
        for value in [Value::Int(7), Value::String("../Other/Model".into())] {
            let mut entity = crate::level::Entity::default();
            entity.properties.insert("stride_model".into(), value);
            assert!(
                validate(&Level {
                    format: 2,
                    generator: "test".into(),
                    name: "test".into(),
                    scale: 32.,
                    textures: vec![],
                    entities: vec![entity]
                })
                .is_err()
            );
        }
    }

    fn glb(json: serde_json::Value, bin: &[u8]) -> Vec<u8> {
        let mut json = serde_json::to_vec(&json).unwrap();
        while !json.len().is_multiple_of(4) {
            json.push(b' ')
        }
        let mut bin = bin.to_vec();
        while !bin.len().is_multiple_of(4) {
            bin.push(0)
        }
        let mut out = b"glTF".to_vec();
        out.extend(2u32.to_le_bytes());
        out.extend(((12 + 8 + json.len() + 8 + bin.len()) as u32).to_le_bytes());
        out.extend((json.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(json);
        out.extend((bin.len() as u32).to_le_bytes());
        out.extend(b"BIN\0");
        out.extend(bin);
        out
    }

    #[test]
    fn every_glb_material_becomes_a_material_and_texture_asset_in_glb_order() {
        let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01";
        let bytes = glb(
            serde_json::json!({
                "asset": {"version": "2.0"},
                "materials": [
                    {"name": "paint", "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}, "metallicFactor": 0.0, "roughnessFactor": 0.25}},
                    {"name": "base/rock", "pbrMetallicRoughness": {"baseColorFactor": [0.5, 0.25, 1.0, 1.0]}, "doubleSided": true}
                ],
                "textures": [{"source": 0, "sampler": 0}],
                "samplers": [{"wrapS": 33071, "wrapT": 33071}],
                "images": [{"bufferView": 0, "mimeType": "image/png"}],
                "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": png.len()}]
            }),
            png,
        );
        let level = Level {
            format: 2,
            generator: "test".into(),
            name: "arena".into(),
            scale: 32.,
            textures: vec![],
            entities: vec![],
        };
        let files = write(
            &level,
            &[],
            &StrideOptions::for_level("arena"),
            &[StrideMesh {
                file_name: "worldspawn.glb",
                glb: &bytes,
            }],
        );
        let model = text(&files, "arena/worldspawn.sdm3d");
        assert!(model.contains("Source: !file worldspawn.glb"), "{model}");
        assert!(
            model.find("Materials/paint").unwrap() < model.find("Materials/base_rock_").unwrap(),
            "{model}"
        );
        assert!(model.starts_with(&format!(
            "!Model\nId: {}\n",
            asset_id("Levels/arena/worldspawn")
        )));
        let paint = text(&files, "Materials/paint.sdmat");
        assert!(
            paint.contains("AddressModeU: Clamp")
                && paint.contains(&format!(
                    "{}:Levels/arena/Textures/paint",
                    asset_id("Levels/arena/Textures/paint")
                )),
            "{paint}"
        );
        assert!(
            paint.contains("Value: 0.75") && !paint.contains("CullMode"),
            "{paint}"
        );
        let rock = files
            .iter()
            .find(|f| f.path.contains("Materials/base_rock_"))
            .unwrap();
        let rock = String::from_utf8(rock.contents.clone()).unwrap();
        assert!(
            rock.contains("!ComputeColor\n            Value: {R: 0.5, G: 0.25, B: 1, A: 1}")
                && rock.contains("CullMode: None"),
            "{rock}"
        );
        assert_eq!(
            files
                .iter()
                .find(|f| f.path == "arena/Textures/paint.png")
                .unwrap()
                .contents,
            png
        );
        assert!(
            text(&files, "Textures/paint.sdtex").contains("GenerateMipmaps: false"),
            "a one-pixel paint has no mipmaps"
        );
        assert_eq!(
            root_assets(&files, &StrideOptions::for_level("arena")),
            [format!(
                "{}:Levels/arena/worldspawn",
                asset_id("Levels/arena/worldspawn")
            )]
        );
    }

    #[test]
    fn the_package_keeps_foreign_roots_and_replaces_only_the_levels_own() {
        let package = "!Package\nAssetFolders:\n    -   Path: !dir Assets\nRootAssets:\n    - AAAA:MainScene\n    - BBBB:Levels/arena/old\n    - CCCC:Levels/arenas/keep\nOther: 1\n";
        let updated = update_package(
            package,
            &["DDDD:Levels/arena/worldspawn".into()],
            "Levels/arena",
        );
        assert_eq!(
            updated,
            "!Package\nAssetFolders:\n    -   Path: !dir Assets\nRootAssets:\n    - AAAA:MainScene\n    - CCCC:Levels/arenas/keep\n    - DDDD:Levels/arena/worldspawn\nOther: 1\n"
        );
        assert_eq!(
            update_package("!Package\nRootAssets: []\n", &["D:L/a/w".into()], "L/a"),
            "!Package\nRootAssets:\n    - D:L/a/w\n"
        );
        assert_eq!(
            update_package("!Package\n", &["D:L/a/w".into()], "L/a"),
            "!Package\nRootAssets:\n    - D:L/a/w\n"
        );
        assert_eq!(
            update_package(
                &updated,
                &["DDDD:Levels/arena/worldspawn".into()],
                "Levels/arena"
            ),
            updated,
            "re-export is stable"
        );
    }

    #[test]
    fn a_sky_becomes_a_cube_texture_a_skybox_and_their_root_assets() {
        let level = Level {
            format: 2,
            generator: "test".into(),
            name: "arena".into(),
            scale: 32.,
            textures: vec![],
            entities: vec![],
        };
        let mut options = StrideOptions::for_level("arena");
        options.sky = Some(crate::writer::sky::Sky {
            sky: [0.1, 0.4, 0.85],
            horizon: [0.7, 0.8, 0.9],
            ground: [0.2, 0.18, 0.15],
            sun_color: [1., 0.9, 0.8],
            sun_intensity: 3.,
            ambient_intensity: 0.5,
            sun_elevation: 90.,
            sun_azimuth: 0.,
            exposure: 1.,
            fog: None,
        });
        let files = write(&level, &[], &options, &[]);
        assert!(text(&files, "Sky/Skybox.sdsky").contains(&format!(
            "CubeMap: {}:Levels/arena/Sky/Sky",
            asset_id("Levels/arena/Sky/Sky")
        )));
        assert!(text(&files, "Sky/Sky.sdtex").contains("UseSRgbSampling: false"));
        assert!(
            files
                .iter()
                .any(|f| f.path == "arena/Sky/Sky.dds" && f.contents.starts_with(b"DDS "))
        );
        let importer = text(&files, "HeronBrushStrideLevel.cs");
        assert!(
            importer.contains("SkyboxUrl = \"Levels/arena/Sky/Skybox\"")
                && importer.contains(",1.000000000f,"),
            "{importer}"
        );
        let roots = root_assets(&files, &options);
        assert!(
            roots
                .iter()
                .any(|r| r.ends_with(":Levels/arena/Sky/Skybox"))
                && roots.iter().any(|r| r.ends_with(":Levels/arena/Sky/Sky"))
        );
        assert!(
            !text(
                &write(&level, &[], &StrideOptions::for_level("arena"), &[]),
                "HeronBrushStrideLevel.cs"
            )
            .contains("Sky/Skybox")
        );
    }

    #[test]
    fn pbr_maps_mask_and_independent_samplers_reach_native_assets() {
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, 4, 4);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[128, 200, 255, 128].repeat(16))
                .unwrap();
        }
        let bytes = glb(
            serde_json::json!({
                "asset": {"version": "2.0"},
                "materials": [{"name": "pbr",
                    "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}, "metallicRoughnessTexture": {"index": 1}, "roughnessFactor": 0.4, "metallicFactor": 0.8},
                    "normalTexture": {"index": 1, "scale": 0.25},
                    "occlusionTexture": {"index": 1, "strength": 0.3},
                    "emissiveTexture": {"index": 1}, "emissiveFactor": [0.2, 0.4, 0.8],
                    "extensions": {"KHR_materials_emissive_strength": {"emissiveStrength": 3.0}},
                    "alphaMode": "MASK", "alphaCutoff": 0.7, "doubleSided": true}],
                "textures": [{"source": 0, "sampler": 0}, {"source": 0, "sampler": 1}],
                "samplers": [{"wrapS": 33071, "wrapT": 10497}, {"wrapS": 10497, "wrapT": 33071}],
                "images": [{"bufferView": 0, "mimeType": "image/png"}],
                "bufferViews": [{"byteLength": png.len()}]
            }),
            &png,
        );
        let level = Level {
            format: 2,
            generator: "test".into(),
            name: "arena".into(),
            scale: 32.,
            textures: vec![],
            entities: vec![],
        };
        let files = write(
            &level,
            &[],
            &StrideOptions::for_level("arena"),
            &[StrideMesh {
                file_name: "world.glb",
                glb: &bytes,
            }],
        );
        let mat = text(&files, "Materials/pbr.sdmat");
        assert!(mat.contains("!MaterialTransparencyCutoffFeature\n        Alpha: !ComputeFloat\n            Value: 0.7"), "{mat}");
        assert!(!mat.contains("!MaterialTransparencyBlendFeature"));
        assert!(
            mat.contains("!MaterialEmissiveMapFeature")
                && mat.contains("Value: 3\n        UseAlpha: false"),
            "{mat}"
        );
        assert!(mat.contains("R: 0.2, G: 0.4, B: 0.8, A: 1"));
        assert!(mat.contains("!MaterialNormalMapFeature") && mat.contains("IsXYNormal: false"));
        assert!(
            mat.contains("R: 0.25, G: 0.25, B: 1, A: 1")
                && mat.contains("R: 0.375, G: 0.375, B: 0.0, A: 0.0"),
            "{mat}"
        );
        assert!(
            mat.contains("Invert: true")
                && mat.contains("Channel: G")
                && mat.contains("Channel: B")
                && mat.contains("Channel: R")
        );
        assert!(
            mat.contains("Value: 0.4")
                && mat.contains("Value: 0.8")
                && mat.contains("Value: 0.3")
                && mat.contains("Value: 0.7")
        );
        assert!(mat.contains("AddressModeU: Clamp\n            AddressModeV: Wrap"));
        assert!(mat.contains("AddressModeU: Wrap") && mat.contains("AddressModeV: Clamp"));
        assert!(
            text(&files, "Textures/pbr.normal.sdtex")
                .contains("Type: !NormalMapTextureType\n    InvertY: true")
        );
        assert!(text(&files, "Textures/pbr.normal.sdtex").contains("IsCompressed: false"));
        for slot in ["mr", "ao"] {
            assert!(
                text(&files, &format!("Textures/pbr.{slot}.sdtex"))
                    .contains("UseSRgbSampling: false")
            );
        }
        for slot in ["", ".normal", ".mr", ".ao", ".emissive"] {
            assert_eq!(
                files
                    .iter()
                    .find(|f| f.path == format!("arena/Textures/pbr{slot}.png"))
                    .unwrap()
                    .contents,
                png
            );
        }
        if let Ok(root) = std::env::var("HERON_STRIDE_MATERIAL_OUTPUT") {
            for file in &files {
                let path = std::path::Path::new(&root).join(&file.path);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, &file.contents).unwrap();
            }
        }
    }

    #[test]
    fn mask_defaults_and_emissive_factors_without_images_are_preserved() {
        let bytes = glb(
            serde_json::json!({"materials": [{"name": "mask", "alphaMode": "MASK", "emissiveFactor": [0.1, 0.2, 0.3]}]}),
            &[],
        );
        let material = glb_materials(&bytes).remove(0);
        let mat = material_asset("test", &material, None, None, None, None, None);
        assert!(mat.contains("!MaterialTransparencyCutoffFeature\n        Alpha: !ComputeFloat\n            Value: 0.5"));
        assert!(mat.contains(
            "EmissiveMap: !ComputeColor\n            Value: {R: 0.1, G: 0.2, B: 0.3, A: 1}"
        ));
        assert!(mat.contains("Intensity: !ComputeFloat\n            Value: 1"));
        assert!(!mat.contains("!MaterialNormalMapFeature"));
    }

    #[test]
    fn same_named_materials_from_different_meshes_do_not_overwrite_each_other() {
        let a = glb(
            serde_json::json!({"materials": [{"name": "Material", "emissiveFactor": [1., 0., 0.]}]}),
            &[],
        );
        let b = glb(
            serde_json::json!({"materials": [{"name": "Material", "alphaMode": "MASK"}]}),
            &[],
        );
        let level = Level {
            format: 2,
            generator: "test".into(),
            name: "arena".into(),
            scale: 32.,
            textures: vec![],
            entities: vec![],
        };
        let meshes = [
            StrideMesh {
                file_name: "a.glb",
                glb: &a,
            },
            StrideMesh {
                file_name: "b.glb",
                glb: &b,
            },
            StrideMesh {
                file_name: "c.glb",
                glb: &a,
            },
        ];
        let options = StrideOptions::for_level("arena");
        let files = write(&level, &[], &options, &meshes);
        let materials: Vec<_> = files
            .iter()
            .filter(|f| f.path.ends_with(".sdmat"))
            .collect();
        assert_eq!(materials.len(), 2);
        assert_ne!(materials[0].path, materials[1].path);
        let model_material = |files: &[StrideFile], path: &str| {
            text(files, path)
                .lines()
                .find(|l| l.trim_start().starts_with("Material:"))
                .unwrap()
                .to_owned()
        };
        assert_eq!(
            model_material(&files, "arena/a.sdm3d"),
            model_material(&files, "arena/c.sdm3d")
        );
        assert_ne!(
            model_material(&files, "arena/a.sdm3d"),
            model_material(&files, "arena/b.sdm3d")
        );
        let reversed = [
            StrideMesh {
                file_name: "b.glb",
                glb: &b,
            },
            StrideMesh {
                file_name: "a.glb",
                glb: &a,
            },
        ];
        let other = write(&level, &[], &options, &reversed);
        assert_eq!(
            model_material(&files, "arena/a.sdm3d"),
            model_material(&other, "arena/a.sdm3d")
        );
        assert_eq!(
            model_material(&files, "arena/b.sdm3d"),
            model_material(&other, "arena/b.sdm3d")
        );
    }

    #[test]
    fn asset_ids_are_stable_guids() {
        let id = asset_id("Levels/arena/worldspawn");
        assert_eq!(id.len(), 36);
        assert_eq!(id, asset_id("Levels/arena/worldspawn"));
        assert_ne!(id, asset_id("Levels/arena/worldspawn_1"));
        assert_eq!(id.matches('-').count(), 4);
    }
}
