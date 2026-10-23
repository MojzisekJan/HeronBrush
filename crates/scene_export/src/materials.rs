//! Finds the material or texture behind each texture name in a map, and reads
//! image dimensions so UVs can be normalised.

use crate::geometry::TextureSizes;
use crate::math::Vec2;
use crate::settings::Settings;
use indexmap::IndexMap;
use std::path::{Path, PathBuf};

/// A texture map slot that gets picked up automatically when a file sits next
/// to the albedo image with the matching suffix.
const PBR_SLOTS: &[(&str, &str)] = &[
    ("_normal", "normal_texture"),
    ("_roughness", "roughness_texture"),
    ("_metallic", "metallic_texture"),
    ("_emission", "emission_texture"),
    ("_ao", "ao_texture"),
    ("_orm", "orm_texture"),
];

#[derive(Debug, Clone)]
pub struct GeneratedMaterial {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub png: Vec<u8>,
    pub alpha: bool,
    pub surface: Option<[f32; 3]>,
    /// The image tiles like an ordinary texture: a texture face given only a surface
    /// material reuses its texture and mapping. A baked face paint covers its face once
    /// and clamps.
    pub repeat: bool,
    pub pbr: Option<GeneratedPbr>,
}

#[derive(Debug, Clone)]
pub struct PbrImage {
    pub slot: &'static str,
    /// Godot has no independent glTF occlusion-strength scalar.
    pub godot_png: Option<std::sync::Arc<[u8]>>,
    pub path: String,
    pub png: std::sync::Arc<[u8]>,
}
/// Source transparency rules, independent of the albedo image contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}

#[derive(Debug, Clone)]
pub struct GeneratedPbr {
    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,
    pub double_sided: bool,
    pub roughness: f32,
    pub metallic: f32,
    pub emissive_factor: [f32; 3],
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub images: Vec<PbrImage>,
}

#[derive(Debug, Clone)]
pub enum Material {
    /// A `.tres` or `.res` the project already has.
    External { res_path: String },
    /// A StandardMaterial3D the writer builds from loose texture files.
    FromTextures {
        albedo: String,
        maps: Vec<(&'static str, String)>,
    },
    /// Nothing on disk matched, so the mesh keeps Godot's default material.
    Missing,
}

#[derive(Debug, Default)]
pub struct MaterialLibrary {
    pub generated: IndexMap<String, GeneratedMaterial>,
    pub materials: IndexMap<String, Material>,
    pub sizes: TextureSizes,
    pub warnings: Vec<String>,
}

impl MaterialLibrary {
    pub fn get(&self, texture: &str) -> Option<&Material> {
        self.materials.get(texture)
    }
}

/// Turns `res://a/b.png` into a path on disk.
pub fn res_to_fs(project_root: &Path, res_path: &str) -> PathBuf {
    project_root.join(res_path.trim_start_matches("res://"))
}

/// Turns a path on disk into `res://a/b.png`, when it sits inside the project.
pub fn fs_to_res(project_root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(project_root).ok()?;
    let mut out = String::from("res://");
    for (i, part) in rel.components().enumerate() {
        if i > 0 {
            out.push('/');
        }
        out.push_str(&part.as_os_str().to_string_lossy());
    }
    Some(out)
}

/// Pulls a texture out of a WAD package and writes it into the project, so
/// Godot has an image to import and the UVs have a size to work from.
///
/// Only the size is essential. The picture needs a palette, and WAD2 shares one
/// with the whole game rather than carrying it, so a Quake package without
/// `--palette` comes out sized but grey.
fn take_from_wad(
    root: &Path,
    name: &str,
    settings: &Settings,
    wads: &[crate::wad::Wad],
    warnings: &mut Vec<String>,
) -> Option<(String, Vec2)> {
    let (wad, entry) = wads.iter().find_map(|w| w.get(name).map(|e| (w, e)))?;
    let size = Vec2::new(entry.width as f64, entry.height as f64);
    if !settings.extract_wad_textures {
        return None;
    }

    let palette = settings.palette.as_deref().and_then(|p| {
        crate::wad::read_palette(p)
            .map_err(|e| warnings.push(e.to_string()))
            .ok()
    });
    let Some(pixels) = wad.pixels(entry, palette.as_ref()) else {
        warnings.push(format!(
            "`{name}` is in {} but there is no palette to colour it with; \
             pass --palette pointing at a palette.lmp",
            wad.path.display()
        ));
        return None;
    };

    let res_path = format!(
        "{}/{}.png",
        settings.texture_dir.trim_end_matches('/'),
        safe_name(name)
    );
    let on_disk = res_to_fs(root, &res_path);
    if let Some(parent) = on_disk.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match write_png(&on_disk, entry.width, entry.height, &pixels) {
        Ok(()) => Some((res_path, size)),
        Err(e) => {
            warnings.push(format!("cannot write {}: {e}", on_disk.display()));
            None
        }
    }
}

/// A texture name Windows will also accept as a file name.
fn safe_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '*' | '?' | '"' | '<' | '>' | '|' | ':' | '\\' => '_',
            other => other,
        })
        .collect()
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(())
}

/// Walks up from `start` looking for the directory holding `project.godot`.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    find_root_marked_by(start, &["project.godot"])
}

/// Walks up from `start` looking for a directory holding any of `markers`.
///
/// The file that says "a project begins here" is the engine's own: `project.godot` for
/// Godot, `game.project` for Defold, a `Cargo.toml` for a Bevy game, and so on. It is
/// what turns a texture name into a file on disk, so an export that cannot find one
/// resolves no material at all and every UV divides by an assumed texture size instead
/// of the real one.
///
/// A marker ending in `/` is a directory rather than a file, which is how Unity says
/// where a project begins.
pub fn find_root_marked_by(start: &Path, markers: &[&str]) -> Option<PathBuf> {
    let absolute = make_absolute(start)?;
    let mut dir = if absolute.is_dir() {
        absolute
    } else {
        absolute.parent()?.to_path_buf()
    };
    loop {
        for marker in markers {
            match marker.strip_suffix('/') {
                Some(name) if dir.join(name).is_dir() => return Some(dir),
                None if dir.join(marker).is_file() => return Some(dir),
                None if marker.starts_with('.')
                    && std::fs::read_dir(&dir).ok().is_some_and(|entries| {
                        entries.flatten().any(|entry| {
                            entry.path().is_file()
                                && entry
                                    .path()
                                    .extension()
                                    .is_some_and(|extension| extension == &marker[1..])
                        })
                    }) =>
                {
                    return Some(dir)
                }
                _ => {}
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Turns a path into an absolute one, ready to be walked up. On Windows that
/// comes back verbatim, beginning `\\?\`, so the prefix comes straight off.
pub fn make_absolute(start: &Path) -> Option<PathBuf> {
    let absolute = start
        .canonicalize()
        .or_else(|_| std::env::current_dir().map(|cwd| cwd.join(start)))
        .ok()?;
    Some(drop_verbatim_prefix(absolute))
}

/// Takes `\\?\` off a plain drive path such as `\\?\C:\project`.
pub fn drop_verbatim_prefix(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    let Some(rest) = text.strip_prefix(r"\\?\") else {
        return path;
    };
    let mut chars = rest.chars();
    let drive = chars.next();
    let colon = chars.next();
    let separator = chars.next();
    let looks_like_a_drive = drive.is_some_and(|c| c.is_ascii_alphabetic())
        && colon == Some(':')
        && matches!(separator, Some('\\') | None);
    if looks_like_a_drive {
        PathBuf::from(rest.to_string())
    } else {
        path
    }
}

/// Resolves every texture name the map uses.
pub fn build(
    textures: &[String],
    settings: &Settings,
    wads: &[crate::wad::Wad],
) -> MaterialLibrary {
    let mut library = MaterialLibrary {
        sizes: TextureSizes::new(settings.fallback_texture_size),
        ..Default::default()
    };

    let remaining: Vec<String> = textures
        .iter()
        .filter_map(|name| {
            if let Some(generated) = settings.generated_materials.get(name) {
                library.generated.insert(name.clone(), generated.clone());
                library.materials.insert(
                    name.clone(),
                    Material::FromTextures {
                        albedo: generated.path.clone(),
                        maps: editor_normal_map(name, settings).into_iter().collect(),
                    },
                );
                library.sizes.insert(
                    name,
                    Vec2::new(generated.width as f64, generated.height as f64),
                );
                None
            } else {
                Some(name.clone())
            }
        })
        .collect();
    let textures = remaining.as_slice();
    if textures.is_empty() {
        return library;
    }

    let Some(root) = settings.project_root.as_deref() else {
        library.warnings.push(
            "no Godot project root, so textures could not be looked up; \
             UVs fall back to the assumed texture size"
                .to_string(),
        );
        for name in textures {
            library.materials.insert(name.clone(), Material::Missing);
        }
        return library;
    };

    for name in textures {
        if settings.is_tool_texture(name) {
            continue;
        }

        if let Some(res_path) = find_external_material(root, name, settings) {
            library
                .materials
                .insert(name.clone(), Material::External { res_path });
            // A hand-made material still needs the image size for correct UVs.
            if let Some((_, size)) = find_texture(root, name, settings) {
                library.sizes.insert(name, size);
            }
            continue;
        }

        // A package knows the size whether or not the picture can be written
        // out, and the size is what the UVs need.
        let in_a_wad = wads
            .iter()
            .find_map(|w| w.get(name))
            .map(|e| Vec2::new(e.width as f64, e.height as f64));
        let found = find_texture(root, name, settings)
            .or_else(|| take_from_wad(root, name, settings, wads, &mut library.warnings));
        match found {
            Some((res_path, size)) => {
                let res_path = match real_spelling(root, &res_path) {
                    Some(real) => {
                        library.warnings.push(format!(
                            "texture `{name}` is spelled `{real}` on disk; using that,                              because a path with the wrong case loads on macOS and                              Windows and loads nothing on Linux"
                        ));
                        real
                    }
                    None => res_path,
                };
                library.sizes.insert(name, size);
                let mut maps = find_pbr_maps(root, name, settings);
                if let Some(normal) = editor_normal_map(name, settings) {
                    maps.retain(|(property, _)| *property != normal.0);
                    maps.push(normal);
                }
                library.materials.insert(
                    name.clone(),
                    Material::FromTextures {
                        albedo: res_path,
                        maps,
                    },
                );
            }
            None => {
                if let Some(size) = in_a_wad {
                    library.sizes.insert(name, size);
                }
                // The placeholder name written on faces that were never textured.
                if name != "__TB_empty" {
                    library
                        .warnings
                        .push(format!("no texture or material found for `{name}`"));
                }
                library.materials.insert(name.clone(), Material::Missing);
            }
        }
    }

    library
}

fn find_external_material(root: &Path, name: &str, settings: &Settings) -> Option<String> {
    let dir = settings.material_dir.as_deref()?;
    for ext in &settings.material_extensions {
        let candidate = res_to_fs(root, &format!("{dir}/{name}.{ext}"));
        if candidate.is_file() {
            return fs_to_res(root, &candidate);
        }
    }
    None
}

/// Looks for `<dir>/<name>.<ext>` and the `<dir>/<name>/<name>.<ext>` layout
/// used when a texture ships with its PBR maps in a folder of its own.
fn find_texture(root: &Path, name: &str, settings: &Settings) -> Option<(String, Vec2)> {
    for stem in [name.to_string(), format!("{name}/{}", leaf(name))] {
        for ext in &settings.texture_extensions {
            let res_path = format!("{}/{stem}.{ext}", settings.texture_dir);
            let candidate = res_to_fs(root, &res_path);
            if candidate.is_file() {
                let size = imagesize::size(&candidate)
                    .map(|s| Vec2::new(s.width as f64, s.height as f64))
                    .unwrap_or(Vec2::splat(settings.fallback_texture_size));
                return Some((res_path, size));
            }
        }
    }
    None
}

/// The spelling a file really has on disk.
///
/// macOS and Windows do not care about case, Linux does. A map naming `Cihly`
/// finds `cihly.png` on a Mac, and the scene then carries a path that loads
/// nothing on Linux.
fn real_spelling(root: &Path, res_path: &str) -> Option<String> {
    let relative = res_path.strip_prefix("res://")?;
    let mut dir = root.to_path_buf();
    let mut out = String::from("res://");
    let mut differs = false;
    for (i, wanted) in relative.split('/').enumerate() {
        let found = std::fs::read_dir(&dir)
            .ok()?
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                entry.file_name().to_string_lossy().to_lowercase() == wanted.to_lowercase()
            })?;
        let real = found.file_name().to_string_lossy().into_owned();
        if real != wanted {
            differs = true;
        }
        if i > 0 {
            out.push('/');
        }
        out.push_str(&real);
        dir = found.path();
    }
    differs.then_some(out)
}

fn find_pbr_maps(root: &Path, name: &str, settings: &Settings) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    for (suffix, property) in PBR_SLOTS {
        'slot: for stem in [
            format!("{name}{suffix}"),
            format!("{name}/{}{suffix}", leaf(name)),
        ] {
            for ext in &settings.texture_extensions {
                let res_path = format!("{}/{stem}.{ext}", settings.texture_dir);
                if res_to_fs(root, &res_path).is_file() {
                    out.push((*property, res_path));
                    break 'slot;
                }
            }
        }
    }
    out
}

/// The normal map the editor draws `name` with, when it handed one over.
fn editor_normal_map(name: &str, settings: &Settings) -> Option<(&'static str, String)> {
    settings
        .normal_map_paths
        .get(name)
        .map(|path| ("normal_texture", path.clone()))
}

fn leaf(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn res_paths_round_trip() {
        let root = Path::new("/project");
        let fs = res_to_fs(root, "res://textures/stone.png");
        assert_eq!(fs, Path::new("/project/textures/stone.png"));
        assert_eq!(
            fs_to_res(root, &fs).as_deref(),
            Some("res://textures/stone.png")
        );
    }

    #[test]
    fn nested_texture_names_keep_their_leaf() {
        assert_eq!(leaf("VOL2/F2/MTFV1_1"), "MTFV1_1");
        assert_eq!(leaf("stone"), "stone");
    }

    #[test]
    fn windows_verbatim_paths_lose_their_prefix() {
        // Written as strings so the check runs on any platform, since this is
        // exactly the case the machine doing the testing does not produce.
        assert_eq!(
            drop_verbatim_prefix(PathBuf::from(r"\\?\C:\project\maps")),
            PathBuf::from(r"C:\project\maps")
        );
        assert_eq!(
            drop_verbatim_prefix(PathBuf::from(r"\\?\C:")),
            PathBuf::from("C:")
        );
        // A share is not a drive. Taking the prefix off changes what it means.
        assert_eq!(
            drop_verbatim_prefix(PathBuf::from(r"\\?\UNC\server\share")),
            PathBuf::from(r"\\?\UNC\server\share")
        );
        // And an ordinary path is left exactly as it was.
        assert_eq!(
            drop_verbatim_prefix(PathBuf::from("/home/someone/project")),
            PathBuf::from("/home/someone/project")
        );
    }

    #[test]
    fn the_project_root_is_found_from_a_relative_path_with_no_parent() {
        let root = std::env::temp_dir().join("heron_brush_root_probe");
        let dir = root.join("maps");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(root.join("project.godot"), "config_version=5").unwrap();
        std::fs::write(dir.join("a.map"), "").unwrap();

        // Standing in maps/, `a.map` has no parent component to walk up from.
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let found = find_project_root(Path::new("a.map"));
        std::env::set_current_dir(previous).unwrap();

        assert_eq!(
            found.map(|p| p.canonicalize().unwrap()),
            Some(root.canonicalize().unwrap())
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_texture_found_under_another_case_is_reported_with_the_name_it_really_has() {
        let root = std::env::temp_dir().join("heron_brush_case_probe");
        let dir = root.join("Textures");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Cihly.png"), []).unwrap();

        // What a map written on a Mac would ask for.
        assert_eq!(
            real_spelling(&root, "res://textures/cihly.png").as_deref(),
            Some("res://Textures/Cihly.png")
        );
        // And the same request spelled right reports nothing to fix.
        assert_eq!(real_spelling(&root, "res://Textures/Cihly.png"), None);
        // Something that is not there at all is not this function's business.
        assert_eq!(real_spelling(&root, "res://Textures/nothing.png"), None);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_editors_normal_map_replaces_the_one_found_by_name() {
        let root = std::env::temp_dir().join("heron_brush_normal_probe");
        let dir = root.join("textures");
        std::fs::create_dir_all(&dir).expect("texture folder");
        std::fs::write(dir.join("stone.png"), []).expect("albedo");
        std::fs::write(dir.join("stone_normal.png"), []).expect("normal");
        let mut settings = Settings {
            project_root: Some(root.clone()),
            texture_dir: "res://textures".into(),
            ..Settings::default()
        };
        let lib = build(&["stone".to_string()], &settings, &[]);
        let Some(Material::FromTextures { maps, .. }) = lib.get("stone") else {
            panic!("stone is not built from textures");
        };
        assert_eq!(
            maps,
            &[(
                "normal_texture",
                "res://textures/stone_normal.png".to_string()
            )]
        );

        // The editor paired `stone_normal_dx` and wrote it out flipped: that one counts.
        settings
            .normal_map_paths
            .insert("stone".into(), "res://level_paint/stone_normal.png".into());
        let lib = build(&["stone".to_string()], &settings, &[]);
        let Some(Material::FromTextures { maps, .. }) = lib.get("stone") else {
            panic!("stone is not built from textures");
        };
        assert_eq!(
            maps,
            &[(
                "normal_texture",
                "res://level_paint/stone_normal.png".to_string()
            )]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn without_a_project_root_every_texture_is_missing_but_the_build_goes_on() {
        let settings = Settings::default();
        let lib = build(&["stone".to_string()], &settings, &[]);
        assert!(matches!(lib.get("stone"), Some(Material::Missing)));
        assert_eq!(lib.sizes.get("stone"), Vec2::splat(32.0));
        assert_eq!(lib.warnings.len(), 1);
    }
}
