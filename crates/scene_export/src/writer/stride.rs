//! Stride 4.4 export: native model, material and texture assets beside the GLBs, plus
//! generated C# that builds the level, adds Bullet colliders and plays animation.
//!
//! Stride compiles assets, not files. A `.glb` dropped into a project is nothing until a
//! `.sdm3d` names it as a model source, and a model draws with the default material until
//! its `Materials` list points at `.sdmat` assets. So every mesh gets a model asset, every
//! glTF material a material asset, and every base-colour image a texture asset, all with
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
        Self { content_dir: format!("Levels/{name}"), sky: None }
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
                if !matches!(value,Value::String(url) if !url.trim().is_empty()&&!url.starts_with('/')&&!url.contains('\\')&&!url.split('/').any(|part|part=="..")) {
                    return Err(format!("{}: {key} must be a project content URL", entity.name));
                }
            }
        }
    }
    Ok(())
}

fn literal(text: &str) -> String {
    serde_json::to_string(text).expect("string literal").replace('\u{2028}', "\\u2028").replace('\u{2029}', "\\u2029")
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
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}
/// The 32-hex item key Stride puts in front of every collection entry.
fn item_key(text: &str) -> String {
    format!("{:016x}{:016x}", fnv(text, 0x9e3779b97f4a7c15), fnv(text, 0xc2b2ae3d27d4eb4f))
}
/// A file-name-safe version of a material name, which can hold `/` or `*`.
fn asset_name(name: &str) -> String {
    let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
    if safe == name { safe } else { format!("{safe}_{:08x}", fnv(name, 0xcbf29ce484222325) as u32) }
}

struct GlbMaterial {
    name: String,
    base_color: [f64; 4],
    metallic: f64,
    roughness: f64,
    double_sided: bool,
    blend: bool,
    image: Option<(Vec<u8>, &'static str)>,
    clamp: bool,
}

fn glb_materials(glb: &[u8]) -> Vec<GlbMaterial> {
    let Some(json_length) = glb.get(12..16).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize) else { return Vec::new() };
    let Some(doc) = glb.get(20..20 + json_length).and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok()) else { return Vec::new() };
    let bin = glb.get(20 + json_length..).and_then(|rest| {
        let length = u32::from_le_bytes(rest.get(0..4)?.try_into().ok()?) as usize;
        rest.get(8..8 + length)
    });
    let image = |texture: &serde_json::Value| -> Option<((Vec<u8>, &'static str), bool)> {
        let texture = &doc["textures"][texture["index"].as_u64()? as usize];
        let clamp = texture["sampler"].as_u64().map(|s| doc["samplers"][s as usize]["wrapS"] == 33071).unwrap_or(false);
        let image = &doc["images"][texture["source"].as_u64()? as usize];
        let view = &doc["bufferViews"][image["bufferView"].as_u64()? as usize];
        let start = view["byteOffset"].as_u64().unwrap_or(0) as usize;
        let bytes = bin?.get(start..start + view["byteLength"].as_u64()? as usize)?.to_vec();
        let extension = if image["mimeType"] == "image/jpeg" { "jpg" } else { "png" };
        Some(((bytes, extension), clamp))
    };
    doc["materials"].as_array().map(|materials| materials.iter().enumerate().map(|(index, m)| {
        let pbr = &m["pbrMetallicRoughness"];
        let base = pbr["baseColorFactor"].as_array().map(|v| [0, 1, 2, 3].map(|i| v.get(i).and_then(|x| x.as_f64()).unwrap_or(1.))).unwrap_or([1.; 4]);
        let found = image(&pbr["baseColorTexture"]);
        GlbMaterial {
            name: m["name"].as_str().map(str::to_owned).unwrap_or_else(|| format!("material_{index}")),
            base_color: base,
            metallic: pbr["metallicFactor"].as_f64().unwrap_or(1.),
            roughness: pbr["roughnessFactor"].as_f64().unwrap_or(1.),
            double_sided: m["doubleSided"].as_bool().unwrap_or(false),
            blend: m["alphaMode"] == "BLEND",
            clamp: found.as_ref().is_some_and(|f| f.1),
            image: found.map(|f| f.0),
        }
    }).collect()).unwrap_or_default()
}

/// Width and height from a PNG's header.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(..8)? != b"\x89PNG\r\n\x1a\n" { return None }
    Some((u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?), u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?)))
}

fn material_asset(id: &str, material: &GlbMaterial, texture: Option<&str>) -> String {
    let [r, g, b, a] = material.base_color;
    let mut text = format!("!MaterialAsset\nId: {id}\nSerializedVersion: {{Stride: 2.0.0.0}}\nTags: []\nAttributes:\n");
    let _ = write!(text, "    MicroSurface: !MaterialGlossinessMapFeature\n        GlossinessMap: !ComputeFloat\n            Value: {}\n", 1. - material.roughness.clamp(0., 1.));
    text.push_str("    Diffuse: !MaterialDiffuseMapFeature\n");
    match texture {
        Some(texture) => {
            let address = if material.clamp { "Clamp" } else { "Wrap" };
            let tint = (r, g, b, a) != (1., 1., 1., 1.);
            let indent = if tint { "                " } else { "            " };
            if tint {
                text.push_str("        DiffuseMap: !ComputeBinaryColor\n            Operator: Multiply\n            LeftChild: !ComputeTextureColor\n");
            } else {
                text.push_str("        DiffuseMap: !ComputeTextureColor\n");
            }
            let _ = write!(text, "{indent}Texture: {texture}\n{indent}FallbackValue:\n{indent}    Value: {{R: 1.0, G: 1.0, B: 1.0, A: 1.0}}\n{indent}AddressModeU: {address}\n{indent}AddressModeV: {address}\n{indent}Scale: {{X: 1.0, Y: 1.0}}\n{indent}Offset: {{X: 0.0, Y: 0.0}}\n{indent}Swizzle: null\n");
            if tint {
                let _ = write!(text, "            RightChild: !ComputeColor\n                Value: {{R: {r}, G: {g}, B: {b}, A: {a}}}\n");
            }
        }
        None => {
            let _ = write!(text, "        DiffuseMap: !ComputeColor\n            Value: {{R: {r}, G: {g}, B: {b}, A: {a}}}\n");
        }
    }
    text.push_str("    DiffuseModel: !MaterialDiffuseLambertModelFeature {}\n");
    let _ = write!(text, "    Specular: !MaterialMetalnessMapFeature\n        MetalnessMap: !ComputeFloat\n            Value: {}\n", material.metallic.clamp(0., 1.));
    text.push_str("    SpecularModel: !MaterialSpecularMicrofacetModelFeature\n        Fresnel: !MaterialSpecularMicrofacetFresnelSchlick {}\n        Visibility: !MaterialSpecularMicrofacetVisibilitySmithSchlickGGX {}\n        NormalDistribution: !MaterialSpecularMicrofacetNormalDistributionGGX {}\n        Environment: !MaterialSpecularMicrofacetEnvironmentGGXLUT {}\n");
    if material.blend {
        // See-through by the diffuse colour's alpha, as glTF's BLEND is.
        text.push_str("    Transparency: !MaterialTransparencyBlendFeature\n        Alpha: !ComputeFloat\n            Value: 1.0\n        Tint: !ComputeColor\n            Value: {R: 1.0, G: 1.0, B: 1.0, A: 1.0}\n");
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
    let models = files.iter().filter(|f| f.path.ends_with(".sdm3d")).map(|f| f.path.rsplit('/').next().unwrap().trim_end_matches(".sdm3d").to_string());
    let sky = files.iter().any(|f| f.path.ends_with("/Sky/Skybox.sdsky")).then(|| ["Sky/Skybox".to_string(), "Sky/Sky".to_string()]).into_iter().flatten();
    models.chain(sky).map(|stem| {
        let url = format!("{prefix}/{stem}");
        format!("{}:{url}", asset_id(&url))
    }).collect()
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
            out.extend(kept.drain(..));
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

pub fn write(level: &Level, collision: &[CollisionMesh], options: &StrideOptions, meshes: &[StrideMesh]) -> Vec<StrideFile> {
    let manifest = super::three::write(level, collision);
    let data: serde_json::Value = serde_json::from_str(&manifest).expect("level JSON");
    let content = options.content_dir.trim_matches('/').to_string();
    let mut files = Vec::new();

    // Materials are shared between the meshes of a level, so each is written once.
    let mut materials: BTreeMap<String, String> = BTreeMap::new();
    for mesh in meshes {
        let stem = mesh.file_name.trim_end_matches(".glb");
        let mut list = String::new();
        for material in glb_materials(mesh.glb) {
            // A see-through copy of a paint (water's surface) is a material of its own,
            // one per colour, beside the paint's opaque one.
            let name = if material.blend {
                asset_name(&format!("{}_blend_{:08x}", material.name, fnv(&format!("{:?}", material.base_color), 0xcbf29ce484222325) as u32))
            } else {
                asset_name(&material.name)
            };
            let url = format!("{content}/Materials/{name}");
            let id = asset_id(&url);
            if !materials.contains_key(&name) {
                let texture = material.image.as_ref().map(|(bytes, extension)| {
                    let texture_url = format!("{content}/Textures/{name}");
                    let texture_id = asset_id(&texture_url);
                    files.push(StrideFile { path: format!("{}/Textures/{name}.{extension}", level.name), contents: bytes.clone() });
                    files.push(StrideFile {
                        path: format!("{}/Textures/{name}.sdtex", level.name),
                        contents: format!("!Texture\nId: {texture_id}\nSerializedVersion: {{Stride: 2.0.0.0}}\nTags: []\nSource: !file {name}.{extension}\n{}Type: !ColorTextureType\n    ColorKeyColor: {{R: 255, G: 0, B: 255, A: 255}}\n",
                            // Stride's converter cannot build mipmaps or compressed blocks
                            // below 4 pixels, which a solid face paint is: one pixel.
                            if png_size(bytes).is_some_and(|(w, h)| w.min(h) < 4) { "IsCompressed: false\nGenerateMipmaps: false\n" } else { "" }).into_bytes(),
                    });
                    format!("{texture_id}:{texture_url}")
                });
                files.push(StrideFile { path: format!("{}/Materials/{name}.sdmat", level.name), contents: material_asset(&id, &material, texture.as_deref()).into_bytes() });
                materials.insert(name.clone(), id.clone());
            }
            let _ = write!(list, "    {}:\n        Name: {}\n        MaterialInstance:\n            Material: {id}:{url}\n", item_key(&format!("{stem}/{}", material.name)), literal(&material.name));
        }
        let url = format!("{content}/{stem}");
        let materials = if list.is_empty() { " {}\n".to_string() } else { format!("\n{list}") };
        files.push(StrideFile {
            path: format!("{}/{stem}.sdm3d", level.name),
            contents: format!("!Model\nId: {}\nSerializedVersion: {{Stride: 2.0.0.0}}\nTags: []\nSource: !file {stem}.glb\nPivotPosition: {{X: 0.0, Y: 0.0, Z: 0.0}}\nMaterials:{materials}Skeleton: null\n", asset_id(&url)).into_bytes(),
        });
    }

    let mut entries = String::new();
    for (index, entity) in level.entities.iter().enumerate() {
        let model = hint(entity, "stride_model").map(str::to_owned).or_else(|| {
            entity.mesh.as_deref().filter(|_| !entity.surfaces.is_empty()).map(|path| format!("{content}/{}", path.rsplit('/').next().unwrap_or(path).trim_end_matches(".glb")))
        });
        let q = glam::DQuat::from_euler(glam::EulerRot::YXZ, entity.transform.rotation_yxz[1], entity.transform.rotation_yxz[0], entity.transform.rotation_yxz[2]);
        let vector = |values: [f64; 3]| format!("new Vector3({:.9}f,{:.9}f,{:.9}f)", values[0], values[1], values[2]);
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
    let mut environment = String::from("        public const string SkyboxUrl = null, SkyTextureUrl = null;\n        public static readonly Color3 SunColor = new Color3(1f,1f,1f);\n        public const float SunIntensity = 0f, AmbientIntensity = 0f;\n        public static readonly Vector3 ToSun = Vector3.UnitY;\n");
    if let Some(sky) = &options.sky {
        let (texture_url, skybox_url) = (format!("{content}/Sky/Sky"), format!("{content}/Sky/Skybox"));
        let texture_id = asset_id(&texture_url);
        // Stride samples a cube with the direction as it is, in its right-handed frame,
        // where Direct3D's cube layout is left-handed: the faces are written mirrored in Z.
        files.push(StrideFile { path: format!("{}/Sky/Sky.dds", level.name), contents: sky.cube_dds(SKY_SIZE, true) });
        files.push(StrideFile { path: format!("{}/Sky/Sky.sdtex", level.name), contents: format!("!Texture\nId: {texture_id}\nSerializedVersion: {{Stride: 2.0.0.0}}\nTags: []\nSource: !file Sky.dds\nIsCompressed: false\nType: !ColorTextureType\n    UseSRgbSampling: false\n    ColorKeyColor: {{R: 255, G: 0, B: 255, A: 255}}\n").into_bytes() });
        files.push(StrideFile { path: format!("{}/Sky/Skybox.sdsky", level.name), contents: format!("!SkyboxAsset\nId: {}\nSerializedVersion: {{Stride: 2.0.0.0}}\nTags: []\nCubeMap: {texture_id}:{texture_url}\n", asset_id(&skybox_url)).into_bytes() });
        let [r, g, b] = sky.sun_color;
        let [x, y, z] = sky.to_sun();
        environment = format!("        public const string SkyboxUrl = {}, SkyTextureUrl = {};\n        public static readonly Color3 SunColor = new Color3({r:.6}f,{g:.6}f,{b:.6}f);\n        public const float SunIntensity = {:.6}f, AmbientIntensity = {:.6}f;\n        public static readonly Vector3 ToSun = new Vector3({x:.9}f,{y:.9}f,{z:.9}f);\n",
            literal(&skybox_url), literal(&texture_url), sky.sun_intensity, sky.ambient_intensity);
    }
    let namespace = format!("HeronBrush.Stride.Level_{:016x}", fnv(&format!("{}\n{}", options.content_dir, level.name), 0xcbf29ce484222325));
    let name = literal(&level.name);
    let fill = |source: &str| {
        let mut script = String::new();
        let mut source = source;
        let replacements = [("__NAMESPACE__", namespace.as_str()), ("__LEVEL__", name.as_str()), ("__ENTRIES__", entries.as_str()), ("__ENVIRONMENT__", environment.as_str())];
        while let Some((index, key, value)) = replacements.iter().filter_map(|(key, value)| source.find(key).map(|index| (index, *key, *value))).min_by_key(|(index, _, _)| *index) {
            script.push_str(&source[..index]);
            script.push_str(value);
            source = &source[index + key.len()..];
        }
        script.push_str(source);
        script.into_bytes()
    };
    files.push(StrideFile { path: format!("{}/HeronBrushStrideLevel.cs", level.name), contents: fill(include_str!("stride_importer.cs")) });
    files.push(StrideFile { path: format!("{}/HeronBrushStrideRuntime.cs", level.name), contents: fill(include_str!("stride_runtime.cs")) });
    files.push(StrideFile { path: format!("{}/HeronBrushStridePhysics.cs", level.name), contents: fill(include_str!("stride_physics.cs")) });
    files.push(StrideFile { path: format!("{}.json", level.name), contents: manifest.into_bytes() });
    files.push(StrideFile { path: "STRIDE-README.md".into(), contents: include_str!("stride_readme.md").into() });
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(files: &[StrideFile], suffix: &str) -> String {
        String::from_utf8(files.iter().find(|f| f.path.ends_with(suffix)).unwrap_or_else(|| panic!("no {suffix}")).contents.clone()).unwrap()
    }

    #[test]
    fn data_only_entities_keep_properties_and_native_placement_without_visual_models() {
        let mut entity = crate::level::Entity { name: "beacon".into(), classname: "puzzle_beacon".into(), ..Default::default() };
        entity.transform.translation = [1., 2., 3.];
        entity.transform.rotation_yxz = [0.37, -1.12, 0.64];
        entity.transform.scale = [2., 0.7, 3.1];
        entity.properties.insert("charge".into(), Value::Int(7));
        entity.properties.insert("stride_prefab".into(), Value::String("Prefabs/Beacon".into()));
        entity.targets.push("later".into());
        let level = Level { format: 2, generator: "test".into(), name: "workshop".into(), scale: 32., textures: vec![], entities: vec![entity] };
        validate(&level).unwrap();
        let files = write(&level, &[], &StrideOptions::for_level("workshop"), &[]);
        let importer = text(&files, "HeronBrushStrideLevel.cs");
        assert!(importer.contains("\"beacon\", null, \"Prefabs/Beacon\""));
        assert!(importer.contains("new Vector3(1.000000000f,2.000000000f,3.000000000f)"));
        let json: serde_json::Value = serde_json::from_str(&text(&files, "workshop.json")).unwrap();
        assert_eq!(json["entities"][0]["properties"]["charge"]["value"], 7);
        assert_eq!(json["entities"][0]["targets"][0], "later");
        assert!(!files.iter().any(|f| f.path.ends_with(".sdm3d")), "no model asset without a mesh");
    }

    #[test]
    fn project_content_hints_reject_invalid_types_and_parent_paths() {
        for value in [Value::Int(7), Value::String("../Other/Model".into())] {
            let mut entity = crate::level::Entity::default();
            entity.properties.insert("stride_model".into(), value);
            assert!(validate(&Level { format: 2, generator: "test".into(), name: "test".into(), scale: 32., textures: vec![], entities: vec![entity] }).is_err());
        }
    }

    fn glb(json: serde_json::Value, bin: &[u8]) -> Vec<u8> {
        let mut json = serde_json::to_vec(&json).unwrap();
        while json.len() % 4 != 0 { json.push(b' ') }
        let mut bin = bin.to_vec();
        while bin.len() % 4 != 0 { bin.push(0) }
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
        let bytes = glb(serde_json::json!({
            "asset": {"version": "2.0"},
            "materials": [
                {"name": "paint", "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}, "metallicFactor": 0.0, "roughnessFactor": 0.25}},
                {"name": "base/rock", "pbrMetallicRoughness": {"baseColorFactor": [0.5, 0.25, 1.0, 1.0]}, "doubleSided": true}
            ],
            "textures": [{"source": 0, "sampler": 0}],
            "samplers": [{"wrapS": 33071, "wrapT": 33071}],
            "images": [{"bufferView": 0, "mimeType": "image/png"}],
            "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": png.len()}]
        }), png);
        let level = Level { format: 2, generator: "test".into(), name: "arena".into(), scale: 32., textures: vec![], entities: vec![] };
        let files = write(&level, &[], &StrideOptions::for_level("arena"), &[StrideMesh { file_name: "worldspawn.glb", glb: &bytes }]);
        let model = text(&files, "arena/worldspawn.sdm3d");
        assert!(model.contains("Source: !file worldspawn.glb"), "{model}");
        assert!(model.find("Materials/paint").unwrap() < model.find("Materials/base_rock_").unwrap(), "{model}");
        assert!(model.starts_with(&format!("!Model\nId: {}\n", asset_id("Levels/arena/worldspawn"))));
        let paint = text(&files, "Materials/paint.sdmat");
        assert!(paint.contains("AddressModeU: Clamp") && paint.contains(&format!("{}:Levels/arena/Textures/paint", asset_id("Levels/arena/Textures/paint"))), "{paint}");
        assert!(paint.contains("Value: 0.75") && !paint.contains("CullMode"), "{paint}");
        let rock = files.iter().find(|f| f.path.contains("Materials/base_rock_")).unwrap();
        let rock = String::from_utf8(rock.contents.clone()).unwrap();
        assert!(rock.contains("!ComputeColor\n            Value: {R: 0.5, G: 0.25, B: 1, A: 1}") && rock.contains("CullMode: None"), "{rock}");
        assert_eq!(files.iter().find(|f| f.path == "arena/Textures/paint.png").unwrap().contents, png);
        assert!(text(&files, "Textures/paint.sdtex").contains("GenerateMipmaps: false"), "a one-pixel paint has no mipmaps");
        assert_eq!(root_assets(&files, &StrideOptions::for_level("arena")), [format!("{}:Levels/arena/worldspawn", asset_id("Levels/arena/worldspawn"))]);
    }

    #[test]
    fn the_package_keeps_foreign_roots_and_replaces_only_the_levels_own() {
        let package = "!Package\nAssetFolders:\n    -   Path: !dir Assets\nRootAssets:\n    - AAAA:MainScene\n    - BBBB:Levels/arena/old\n    - CCCC:Levels/arenas/keep\nOther: 1\n";
        let updated = update_package(package, &["DDDD:Levels/arena/worldspawn".into()], "Levels/arena");
        assert_eq!(updated, "!Package\nAssetFolders:\n    -   Path: !dir Assets\nRootAssets:\n    - AAAA:MainScene\n    - CCCC:Levels/arenas/keep\n    - DDDD:Levels/arena/worldspawn\nOther: 1\n");
        assert_eq!(update_package("!Package\nRootAssets: []\n", &["D:L/a/w".into()], "L/a"), "!Package\nRootAssets:\n    - D:L/a/w\n");
        assert_eq!(update_package("!Package\n", &["D:L/a/w".into()], "L/a"), "!Package\nRootAssets:\n    - D:L/a/w\n");
        assert_eq!(update_package(&updated, &["DDDD:Levels/arena/worldspawn".into()], "Levels/arena"), updated, "re-export is stable");
    }

    #[test]
    fn a_sky_becomes_a_cube_texture_a_skybox_and_their_root_assets() {
        let level = Level { format: 2, generator: "test".into(), name: "arena".into(), scale: 32., textures: vec![], entities: vec![] };
        let mut options = StrideOptions::for_level("arena");
        options.sky = Some(crate::writer::sky::Sky { sky: [0.1, 0.4, 0.85], horizon: [0.7, 0.8, 0.9], ground: [0.2, 0.18, 0.15], sun_color: [1., 0.9, 0.8], sun_intensity: 3., ambient_intensity: 0.5, sun_elevation: 90., sun_azimuth: 0., exposure: 1., fog: None });
        let files = write(&level, &[], &options, &[]);
        assert!(text(&files, "Sky/Skybox.sdsky").contains(&format!("CubeMap: {}:Levels/arena/Sky/Sky", asset_id("Levels/arena/Sky/Sky"))));
        assert!(text(&files, "Sky/Sky.sdtex").contains("UseSRgbSampling: false"));
        assert!(files.iter().any(|f| f.path == "arena/Sky/Sky.dds" && f.contents.starts_with(b"DDS ")));
        let importer = text(&files, "HeronBrushStrideLevel.cs");
        assert!(importer.contains("SkyboxUrl = \"Levels/arena/Sky/Skybox\"") && importer.contains(",1.000000000f,"), "{importer}");
        let roots = root_assets(&files, &options);
        assert!(roots.iter().any(|r| r.ends_with(":Levels/arena/Sky/Skybox")) && roots.iter().any(|r| r.ends_with(":Levels/arena/Sky/Sky")));
        assert!(!text(&write(&level, &[], &StrideOptions::for_level("arena"), &[]), "HeronBrushStrideLevel.cs").contains("Sky/Skybox"));
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
