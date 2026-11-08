//! Optional Defold editor commands and a managed level lifecycle in the existing game world.
use crate::{
    document::Document,
    export::{self, Options, Target},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
/// Reserved addon resources inside the Defold project.
pub const FOLDER: &str = "heron_brush";
/// Project-owned source map and export configuration.
pub const CONFIG: &str = "heron_brush.json";
const MARKER: &str = "Heron Brush Defold addon v1\n";

/// Settings shared by the Defold editor commands.
#[derive(Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Source map, absolute or relative to game.project.
    pub map: String,
    /// Optional entity definitions, using the same path rules as the map.
    pub fgd: String,
    /// Optional material source directory.
    pub materials: String,
    /// Partial export options; target and preview resource paths are fixed.
    pub options: serde_json::Value,
}
fn root(path: &Path) -> Result<PathBuf, String> {
    let root = path.canonicalize().map_err(|e| e.to_string())?;
    if !root.join("game.project").is_file() {
        return Err("Not a Defold project".into());
    }
    Ok(root)
}
/// Whether the project has an owned addon installation.
pub fn installed(path: &Path) -> bool {
    if safe_destination(path, &path.join(FOLDER).join(".heron-brush-addon")).is_err() {
        return false;
    }
    std::fs::read_to_string(path.join(FOLDER).join(".heron-brush-addon"))
        .ok()
        .as_deref()
        == Some(MARKER)
}
/// Install or update owned addon files while preserving the project configuration.
pub fn install(path: &Path, options: &Options, document: Option<&Document>) -> Result<(), String> {
    let root = root(path)?;
    let folder = root.join(FOLDER);
    if folder.exists()
        && (folder.canonicalize().map_err(|e| e.to_string())? != folder || !installed(&root))
    {
        return Err(
            "The heron_brush folder is not owned by this addon; move it aside first".into(),
        );
    }
    let files = [
        (
            "heron_brush.editor_script",
            include_str!("defold_addon/heron_brush.editor_script"),
        ),
        (
            "controller.script",
            include_str!("defold_addon/controller.script"),
        ),
        ("controller.go", include_str!("defold_addon/controller.go")),
        (
            "source.collectionfactory",
            include_str!("defold_addon/source.collectionfactory"),
        ),
        (
            "bootstrap.collection",
            include_str!("defold_addon/bootstrap.collection"),
        ),
        (
            "level.collectionfactory",
            include_str!("defold_addon/level.collectionfactory"),
        ),
    ];
    // Check all paths, including broken symbolic links, before changing any files.
    for name in files
        .iter()
        .map(|(name, _)| *name)
        .chain([".heron-brush-addon"])
    {
        safe_destination(&root, &folder.join(name))?;
    }
    safe_destination(&root, &root.join(CONFIG))?;
    for name in ["preview.collection", "revision.script"] {
        safe_destination(&root, &root.join(crate::defold_live::FOLDER).join(name))?;
    }
    crate::defold_live::claim_folder(&root)?;
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    for (name, text) in files {
        crate::files::write_atomic(&folder.join(name), text).map_err(|e| e.to_string())?;
    }
    crate::files::write_atomic(&folder.join(".heron-brush-addon"), MARKER)
        .map_err(|e| e.to_string())?;
    if !root.join(CONFIG).exists() {
        let mut options = options.clone();
        options.target = Target::Defold;
        options.asset_dir.clear();
        options.directory = crate::defold_live::FOLDER.into();
        options.name = crate::defold_live::NAME.into();
        options.project_dir = ".".into();
        let config = Config {
            map: document
                .and_then(|d| d.path.as_ref())
                .map(|p| {
                    p.canonicalize()
                        .unwrap_or_else(|_| p.to_path_buf())
                        .to_string_lossy()
                        .into_owned()
                })
                .unwrap_or_default(),
            fgd: document
                .and_then(Document::definitions_path)
                .map(|p| {
                    p.canonicalize()
                        .unwrap_or_else(|_| p.to_path_buf())
                        .to_string_lossy()
                        .into_owned()
                })
                .unwrap_or_default(),
            materials: String::new(),
            options: serde_json::to_value(options).map_err(|e| e.to_string())?,
        };
        crate::files::write_atomic(
            &root.join(CONFIG),
            serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    let preview = root.join(crate::defold_live::FOLDER);
    if !preview.join("preview.collection").exists() {
        crate::files::write_atomic(
            &preview.join("preview.collection"),
            "name: \"heron_preview\"\n",
        )
        .map_err(|e| e.to_string())?;
    }
    if !preview.join("revision.script").exists() {
        crate::files::write_atomic(
            &preview.join("revision.script"),
            revision_script("installed"),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
/// A changed script resource turns Defold hot reload into a managed level reload.
pub fn revision_script(revision: &str) -> String {
    revision_with_receipt(revision, "", 0)
}
pub(crate) fn revision_with_receipt(
    revision: &str,
    ack_url: &str,
    minimum_objects: usize,
) -> String {
    format!("local revision = {revision:?}\nlocal ack_url = {ack_url:?}\nfunction on_reload(self) self.heron_revision = revision; msg.post(\"#controller\", \"heron_reload_level\", {{ack_url=ack_url,minimum_objects={minimum_objects}}}) end\n")
}

fn options(root: &Path, config: &Config) -> Result<Options, String> {
    options_with_base(root, config, &Options::default())
}
/// Apply the same project overrides to live snapshots and saved-map exports.
pub(crate) fn live_options(root: &Path, base: &Options) -> Result<Options, String> {
    if !installed(root) {
        return Ok(base.clone());
    }
    let config: Config =
        serde_json::from_slice(&std::fs::read(root.join(CONFIG)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    options_with_base(root, &config, base)
}
fn options_with_base(root: &Path, config: &Config, base: &Options) -> Result<Options, String> {
    let mut value = serde_json::to_value(base).map_err(|e| e.to_string())?;
    if let Some(overrides) = config.options.as_object() {
        for (key, v) in overrides {
            value[key] = v.clone();
        }
    } else if !config.options.is_null() {
        return Err("options must be a JSON object".into());
    }
    let mut options: Options = serde_json::from_value(value).map_err(|e| e.to_string())?;
    options.target = Target::Defold;
    options.project_dir = root.to_string_lossy().into_owned();
    options.directory = root
        .join(crate::defold_live::FOLDER)
        .to_string_lossy()
        .into_owned();
    options.name = crate::defold_live::NAME.into();
    options.asset_dir.clear();
    crate::export_bindings::validate(&options.entities)?;
    for rule in options.entities.values() {
        validate_resources(
            root,
            &serde_json::to_value(&rule.defaults).map_err(|e| e.to_string())?,
        )?;
    }
    Ok(options)
}
fn publish(root: &Path, document: &Document, options: &Options) -> Result<(), String> {
    crate::defold_live::claim_folder(root)?;
    let mut options = options.clone();
    options.target = Target::Defold;
    options.project_dir = root.to_string_lossy().into();
    options.directory = root
        .join(crate::defold_live::FOLDER)
        .to_string_lossy()
        .into();
    options.name = crate::defold_live::NAME.into();
    options.asset_dir.clear();
    let (_, files) = export::export_files(document, Path::new(&options.directory), &options)
        .map_err(|e| e.to_string())?;
    validate_files(root, &files)?;
    let _lock = crate::defold_live::publication_lock(root)?;
    if root.join(".heron-brush-live.json").exists() {
        return Err("Stop the live link before re-exporting the saved source map".into());
    }
    for (path, contents) in files {
        let bytes = match contents {
            export::Contents::Text(s) => s.into_bytes(),
            export::Contents::Bytes(b) => b,
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        crate::files::write_atomic(&path, bytes).map_err(|e| e.to_string())?;
    }
    bump_revision(root)
}
fn bump_revision(root: &Path) -> Result<(), String> {
    safe_destination(
        root,
        &root
            .join(crate::defold_live::FOLDER)
            .join("revision.script"),
    )?;
    let revision = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    );
    crate::files::write_atomic(
        &root
            .join(crate::defold_live::FOLDER)
            .join("revision.script"),
        revision_script(&revision),
    )
    .map_err(|e| e.to_string())
}
/// Run a headless project action from the editor addon or command line.
pub fn command(args: &[String]) -> Result<(), String> {
    let [path, action] = args else {
        return Err("Usage: --defold-project <project> install|open|export|validate|reload".into());
    };
    if !matches!(
        action.as_str(),
        "install" | "open" | "export" | "validate" | "reload"
    ) {
        return Err("Unknown Defold project action".into());
    }
    let root = root(Path::new(path))?;
    if action == "install" {
        return install(&root, &Options::default(), None);
    }
    if !installed(&root) {
        return Err("Install Heron Brush support in this Defold project first".into());
    }
    if action == "reload" {
        {
            let _lock = crate::defold_live::publication_lock(&root)?;
            bump_revision(&root)?;
        }
        println!("{}", crate::defold_live::refresh(&root, true)?);
        return Ok(());
    }
    let config: Config =
        serde_json::from_slice(&std::fs::read(root.join(CONFIG)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if config.map.is_empty() {
        return Err("Choose a source map in Heron Brush settings".into());
    }
    let resolve = |s: &str| {
        if Path::new(s).is_absolute() {
            PathBuf::from(s)
        } else {
            root.join(s)
        }
    };
    let map = resolve(&config.map);
    if action == "open" {
        std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
            .arg(map)
            .arg(if config.fgd.is_empty() {
                PathBuf::new()
            } else {
                resolve(&config.fgd)
            })
            .arg(if config.materials.is_empty() {
                PathBuf::new()
            } else {
                resolve(&config.materials)
            })
            // The editor captures this command's output. The GUI must not keep that pipe open.
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if !matches!(action.as_str(), "export" | "validate") {
        return Err("Unknown Defold project action".into());
    }
    let mut document = Document::open(map, Default::default()).map_err(|e| e.to_string())?;
    if !config.fgd.is_empty() {
        document
            .load_definitions(resolve(&config.fgd))
            .map_err(|e| e.to_string())?;
    }
    if !config.materials.is_empty() {
        document
            .load_materials(resolve(&config.materials))
            .map_err(|e| e.to_string())?;
    }
    let options = options(&root, &config)?;
    if action == "validate" {
        let (_, files) = export::export_files(&document, Path::new(&options.directory), &options)
            .map_err(|e| e.to_string())?;
        validate_files(&root, &files)?;
        println!("Heron Brush: export and bindings are valid");
    } else {
        publish(&root, &document, &options)?;
        println!("Heron Brush: preview exported; use Reload Level for the running game");
    }
    Ok(())
}

// Check existing ancestors individually: canonicalizing only the full path misses dangling links.
fn safe_destination(root: &Path, path: &Path) -> Result<(), String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "Export resource is outside the Defold project")?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err("Export resource has an invalid path".into());
        }
        current.push(component.as_os_str());
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "Generated resource must not be a symbolic link: {}",
                    current.display()
                ))
            }
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}
pub(crate) fn validate_resources(root: &Path, value: &serde_json::Value) -> Result<(), String> {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if key == "defold_go" || key.starts_with("defold_component_") {
                    let resource = value
                        .as_str()
                        .or_else(|| {
                            (value.get("type").and_then(serde_json::Value::as_str)
                                == Some("string"))
                            .then(|| value.get("value").and_then(serde_json::Value::as_str))
                            .flatten()
                        })
                        .ok_or_else(|| format!("{key} must be a resource path"))?;
                    let relative = resource
                        .strip_prefix('/')
                        .ok_or_else(|| format!("{key} must start with /"))?;
                    if !Path::new(relative)
                        .components()
                        .all(|c| matches!(c, std::path::Component::Normal(_)))
                    {
                        return Err(format!("Invalid resource path: {resource}"));
                    }
                    if !relative.starts_with("builtins/") {
                        let path = root
                            .join(relative)
                            .canonicalize()
                            .map_err(|_| format!("Missing Defold resource: {resource}"))?;
                        if !path.starts_with(root) || !path.is_file() {
                            return Err(format!("Invalid Defold resource: {resource}"));
                        }
                    }
                }
                validate_resources(root, value)?;
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                validate_resources(root, value)?;
            }
        }
        _ => (),
    }
    Ok(())
}
fn validate_files(root: &Path, files: &[(PathBuf, export::Contents)]) -> Result<(), String> {
    for (path, contents) in files {
        safe_destination(root, path)?;
        if path.extension().is_some_and(|e| e == "json") {
            if let export::Contents::Text(text) = contents {
                let value: serde_json::Value =
                    serde_json::from_str(text).map_err(|e| e.to_string())?;
                validate_resources(root, &value)?;
            }
        }
    }
    safe_destination(
        root,
        &root
            .join(crate::defold_live::FOLDER)
            .join("revision.script"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Project(PathBuf);
    impl Project {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "heron-addon-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join("game.project"), "[project]\ntitle = Test\n").unwrap();
            Self(path.canonicalize().unwrap())
        }
    }
    impl Drop for Project {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn install_preserves_project_and_config_and_refuses_unowned_folders() {
        let p = Project::new();
        let game = std::fs::read(p.0.join("game.project")).unwrap();
        let config = br#"{"map":"maps/level.map","options":{"scale":16}}"#;
        std::fs::write(p.0.join(CONFIG), config).unwrap();
        install(&p.0, &Options::default(), None).unwrap();
        assert!(installed(&p.0));
        install(&p.0, &Options::default(), None).unwrap();
        assert_eq!(std::fs::read(p.0.join(CONFIG)).unwrap(), config);
        assert_eq!(std::fs::read(p.0.join("game.project")).unwrap(), game);
        assert!(p.0.join(FOLDER).join("controller.go").is_file());
        let other = Project::new();
        std::fs::create_dir(other.0.join(FOLDER)).unwrap();
        std::fs::write(
            other.0.join(FOLDER).join("controller.script"),
            "user script",
        )
        .unwrap();
        assert!(install(&other.0, &Options::default(), None).is_err());
        assert_eq!(
            std::fs::read_to_string(other.0.join(FOLDER).join("controller.script")).unwrap(),
            "user script"
        );
    }
    #[test]
    fn partial_settings_are_merged_and_unknown_settings_are_rejected() {
        let p = Project::new();
        let config: Config = serde_json::from_value(serde_json::json!({"options":{"scale":16,"target":"Godot","name":"custom","asset_dir":"/wrong"}})).unwrap();
        let options = options(&p.0, &config).unwrap();
        assert_eq!(options.target, Target::Defold);
        assert_eq!(options.name, "preview");
        assert!(options.asset_dir.is_empty());
        install(&p.0, &Options::default(), None).unwrap();
        std::fs::write(p.0.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
        assert_eq!(live_options(&p.0, &Options::default()).unwrap().scale, 16.0);
        let bad: Config =
            serde_json::from_value(serde_json::json!({"options":{"scsale":16}})).unwrap();
        assert!(super::options(&p.0, &bad).is_err());
    }
    #[test]
    fn binding_resources_must_exist_inside_project() {
        let p = Project::new();
        let value = serde_json::json!({"entities":[{"properties":{"defold_component_logic":"/logic.script"}}]});
        assert!(validate_resources(&p.0, &value)
            .unwrap_err()
            .contains("Missing"));
        std::fs::write(p.0.join("logic.script"), "function init(self) end").unwrap();
        validate_resources(&p.0, &value).unwrap();
        validate_resources(&p.0, &serde_json::json!({"properties":{"defold_component_logic":{"type":"string","value":"/logic.script"}}})).unwrap();
        assert!(
            validate_resources(&p.0, &serde_json::json!({"defold_go":"/../outside.go"})).is_err()
        );
        assert!(safe_destination(&p.0, &p.0.join("../outside.go")).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_are_rejected_before_install_or_publication() {
        let p = Project::new();
        let other = Project::new();
        install(&p.0, &Options::default(), None).unwrap();
        let script = p.0.join(FOLDER).join("controller.script");
        std::fs::remove_file(&script).unwrap();
        std::os::unix::fs::symlink(other.0.join("missing"), &script).unwrap();
        assert!(install(&p.0, &Options::default(), None).is_err());
        assert!(!other.0.join("missing").exists());
        std::os::unix::fs::symlink(&other.0, p.0.join(crate::defold_live::FOLDER).join("mesh"))
            .unwrap();
        assert!(validate_files(
            &p.0,
            &[(
                p.0.join(crate::defold_live::FOLDER).join("mesh/file"),
                export::Contents::Text("data".into())
            )]
        )
        .is_err());
        let marker = p.0.join(FOLDER).join(".heron-brush-addon");
        std::fs::remove_file(&marker).unwrap();
        std::os::unix::fs::symlink(other.0.join("missing"), marker).unwrap();
        assert!(!installed(&p.0));
    }
    #[test]
    fn opaque_imported_albedo_keeps_rgb_even_at_zero_source_alpha() {
        use scene_export::materials::{AlphaMode, GeneratedMaterial, GeneratedPbr};
        let p = Project::new();
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([123, 45, 67, 0]),
        ))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
        let mut settings = scene_export::settings::Settings::default();
        settings.generated_materials.insert(
            "paint".into(),
            GeneratedMaterial {
                path: "paint".into(),
                width: 1,
                height: 1,
                png,
                alpha: true,
                surface: None,
                repeat: false,
                pbr: Some(GeneratedPbr {
                    alpha_mode: AlphaMode::Opaque,
                    alpha_cutoff: 0.5,
                    double_sided: false,
                    roughness: 1.,
                    metallic: 0.,
                    emissive_factor: [0.; 3],
                    normal_scale: 1.,
                    occlusion_strength: 1.,
                    layers: [0.0; 4],
                    images: vec![],
                }),
            },
        );
        let level = scene_export::level::Level {
            format: 2,
            generator: "test".into(),
            name: "test".into(),
            scale: 32.,
            textures: vec![],
            entities: vec![],
        };
        let mut options = scene_export::writer::defold::DefoldOptions::default();
        let files = crate::export::configure_defold_materials(
            &level,
            &settings,
            &p.0,
            "test",
            &p.0,
            &mut options,
        )
        .unwrap();
        let pixel = image::load_from_memory(&files[0].1).unwrap().to_rgba8()[(0, 0)];
        assert_eq!(pixel.0, [123, 45, 67, 255]);
        assert!(options.opaque_albedos.contains_key("paint"));
    }
    #[test]
    fn cli_rejects_missing_or_unknown_actions() {
        assert!(command(&[]).is_err());
        assert!(command(&["missing".into(), "typo".into()])
            .unwrap_err()
            .contains("Unknown"));
        assert!(matches!(
            crate::cli::parse(&["--defold-project".into(), ".".into(), "export".into()]),
            crate::cli::Command::DefoldProject(_)
        ));
    }
    #[test]
    #[ignore = "requires native rendering verification"]
    fn native_render_fixture() {
        use geometry_math::{bbox::Bbox3d, vec::Vec3d};
        let root = PathBuf::from(std::env::var("HERON_DEFOLD_RENDER_PROJECT").unwrap());
        std::fs::create_dir_all(&root).unwrap();
        assert!(!root.join("game.project").exists());
        std::fs::write(root.join("game.project"), "[project]\ntitle = Heron rendering check\n[bootstrap]\nmain_collection = /main.collectionc\nrender = /heron_brush_live/preview.renderc\n[physics]\ntype = 3D\nscale = 1\n[display]\nwidth = 320\nheight = 240\nvsync = 1\nupdate_frequency = 60\n").unwrap();
        let root = root.canonicalize().unwrap();
        let mut doc = Document::new(Default::default());
        doc.add_brushes(vec![map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &Bbox3d::new(Vec3d::new(-32., -32., -32.), Vec3d::new(32., 32., 32.)),
            "stone",
        )
        .unwrap()]);
        let source=doc.to_map_text()+"\n{\n\"classname\" \"light\"\n\"origin\" \"0 -96 0\"\n\"light_color\" \"255 0 0\"\n\"light_energy\" \"30\"\n\"omni_range\" \"10\"\n}\n";
        std::fs::write(root.join("source.map"), source).unwrap();
        let mut doc = Document::open(root.join("source.map"), Default::default()).unwrap();
        let model_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/editor-checks/fixtures/pbr.glb");
        std::fs::copy(model_source, root.join("pbr.glb")).unwrap();
        doc.import_model(&root.join("pbr.glb"), Vec3d::new(320., 320., 320.))
            .unwrap();
        doc.save_as(root.join("source.map")).unwrap();
        let opts = Options {
            target: Target::Defold,
            sky: false,
            ..Default::default()
        };
        install(&root, &opts, Some(&doc)).unwrap();
        command(&[root.to_string_lossy().into(), "export".into()]).unwrap();
    }
    /// Creates a disposable project for examples/engine-checks/defold/verify_addon.py.
    #[test]
    #[ignore]
    fn native_addon_fixture() {
        use geometry_math::{bbox::Bbox3d, vec::Vec3d};
        let root = PathBuf::from(
            std::env::var("HERON_DEFOLD_ADDON_PROJECT").expect("isolated project directory"),
        );
        std::fs::create_dir_all(&root).unwrap();
        assert!(
            !root.join("game.project").exists(),
            "fixture requires a new project"
        );
        std::fs::write(root.join("game.project"), "[project]\ntitle = Heron addon verification\n[bootstrap]\nmain_collection = /main.collectionc\n[physics]\ntype = 3D\nscale = 1\n[display]\nwidth = 320\nheight = 240\nvsync = 1\nupdate_frequency = 60\n").unwrap();
        let root = root.canonicalize().unwrap();
        let mut doc = Document::new(Default::default());
        doc.add_brushes(vec![map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &Bbox3d::new(
                Vec3d::new(-32.0, -32.0, -32.0),
                Vec3d::new(32.0, 32.0, 32.0),
            ),
            "stone",
        )
        .unwrap()]);
        std::fs::write(root.join("source.map"), doc.to_map_text()).unwrap();
        doc.path = Some(root.join("source.map"));
        std::fs::write(root.join("logic.script"), "function init(self) end\n").unwrap();
        let opts = Options {
            target: Target::Defold,
            sky: false,
            entities: [(
                "worldspawn".into(),
                crate::export_bindings::Rule {
                    defaults: [(
                        "defold_component_logic".into(),
                        serde_json::json!("/logic.script"),
                    )]
                    .into(),
                    ..Default::default()
                },
            )]
            .into(),
            ..Default::default()
        };
        install(&root, &opts, Some(&doc)).unwrap();
        command(&[root.to_string_lossy().into(), "validate".into()]).unwrap();
        command(&[root.to_string_lossy().into(), "export".into()]).unwrap();
    }
}
