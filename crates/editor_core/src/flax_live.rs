//! Flax live snapshots and installation of the editor-side scene importer.
use crate::{
    export::Options,
    godot_live::{atomic_write, Prepared},
};
use std::path::{Path, PathBuf};
pub const FOLDER: &str = "Content/HeronBrushLive";
pub fn project(options: &Options) -> Result<PathBuf, String> {
    let directory = Path::new(&options.directory)
        .canonicalize()
        .map_err(|_| "Export into a Flax project first.")?;
    let marker = |p: &Path| {
        p.read_dir().ok().is_some_and(|entries| {
            entries
                .flatten()
                .any(|e| e.path().extension().is_some_and(|x| x == "flaxproj"))
        })
    };
    let root = if options.project_dir.is_empty() {
        directory
            .ancestors()
            .find(|p| marker(p))
            .ok_or("No Flax project found above the export directory.")?
            .to_path_buf()
    } else {
        Path::new(&options.project_dir)
            .canonicalize()
            .map_err(|e| e.to_string())?
    };
    if !marker(&root) || !directory.starts_with(root.join("Content")) {
        return Err("Choose an export folder inside the Flax project's Content directory.".into());
    }
    Ok(root)
}
fn module(root: &Path) -> Result<PathBuf, String> {
    let source = root.join("Source");
    if source.join("Game/Game.Build.cs").is_file() {
        return Ok(source.join("Game/HeronBrushLive"));
    }
    let modules: Vec<_> = source
        .read_dir()
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.join(format!(
                    "{}.Build.cs",
                    p.file_name().unwrap().to_string_lossy()
                ))
                .is_file()
        })
        .collect();
    if modules.len() != 1 {
        return Err("Could not choose a Flax game module. Install the addon into the project's Game module.".into());
    }
    Ok(modules[0].join("HeronBrushLive"))
}
fn addon() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (
            "HeronBrushLevelRoot.cs",
            include_bytes!("../../scene_export/src/writer/flax_level_root.cs").to_vec(),
        ),
        (
            "HeronBrushLiveBuilder.cs",
            scene_export::writer::flax::live_importer().into_bytes(),
        ),
        (
            "HeronBrushLiveEditor.cs",
            include_bytes!("flax_live/HeronBrushLiveEditor.cs").to_vec(),
        ),
        (
            "HeronBrushLiveAnchor.cs",
            include_bytes!("flax_live/HeronBrushLiveAnchor.cs").to_vec(),
        ),
    ]
}
const OWNERSHIP: &str = ".heron-addon.json";
fn fingerprint(bytes: &[u8]) -> String {
    crate::export::paint_export::image_name(bytes)
}
pub fn installed(root: &Path) -> bool {
    module(root).is_ok_and(|p| {
        addon()
            .iter()
            .all(|(name, bytes)| std::fs::read(p.join(name)).is_ok_and(|current| current == *bytes))
    })
}
pub fn install(root: &Path) -> Result<(), String> {
    let module = module(root)?;
    let files = addon();
    let previous: std::collections::BTreeMap<String, String> =
        match std::fs::read(module.join(OWNERSHIP)) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| format!("Invalid addon ownership manifest: {e}"))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Default::default(),
            Err(error) => return Err(error.to_string()),
        };
    // Validate every existing file before replacing any generated source.
    for (name, bytes) in &files {
        let path = module.join(name);
        if path.exists() {
            let current = std::fs::read(&path).map_err(|e| e.to_string())?;
            if current != *bytes && previous.get(*name) != Some(&fingerprint(&current)) {
                return Err(format!(
                    "{} has local edits. Move it aside before installing the addon.",
                    path.display()
                ));
            }
        }
    }
    let ownership: std::collections::BTreeMap<_, _> = files
        .iter()
        .map(|(name, bytes)| (*name, fingerprint(bytes)))
        .collect();
    for (name, bytes) in files {
        atomic_write(&module.join(name), &bytes).map_err(|e| e.to_string())?;
    }
    atomic_write(
        &module.join(OWNERSHIP),
        &serde_json::to_vec(&ownership).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn claim_folder(root: &Path) -> Result<(), String> {
    let folder = root.join(FOLDER);
    let marker = folder.join(".heron-brush-live");
    if folder.exists() {
        if folder.canonicalize().map_err(|e| e.to_string())? != folder || !marker.is_file() {
            return Err("Content/HeronBrushLive already exists and is not owned by the live link. Move it aside first.".into());
        }
    } else {
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        atomic_write(&marker, b"Heron Brush live v1").map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn prepare(
    text: &str,
    classes: &scene_export::indexmap::IndexMap<String, scene_export::fgd::EntityClass>,
    settings: &scene_export::settings::Settings,
    options: &Options,
    root: &Path,
) -> Result<Prepared, String> {
    let map = scene_export::map::parse(text, "preview").map_err(|e| e.to_string())?;
    let classes = crate::export_bindings::classes(&map, classes, &options.entities)?;
    let mut built = scene_export::builder::build_level(
        &map,
        &classes,
        settings,
        &scene_export::builder::LevelOptions {
            name: "preview".into(),
            mesh_dir: "preview".into(),
            collision: options.collision,
            inline_collision: true,
            mesh_collision: false,
            convex_collision: false,
            mesh_format: scene_export::builder::MeshFormat::Glb,
        },
    );
    scene_export::writer::flax::validate(&built.level)?;
    let directory = root.join(FOLDER);
    let mut files = Vec::new();
    let mut meshes = Vec::new();
    for mesh in built.meshes {
        // Immutable asset paths keep the previous scene intact while the next imports.
        let name = format!(
            "{}.glb",
            crate::export::paint_export::image_name(&mesh.bytes)
        );
        for entity in &mut built.level.entities {
            if entity
                .mesh
                .as_ref()
                .is_some_and(|p| p.rsplit('/').next() == Some(mesh.file_name.as_str()))
            {
                entity.mesh = Some(format!("preview/{name}"));
            }
        }
        let path = directory.join("preview").join(&name);
        meshes.push(format!("{FOLDER}/preview/{name}"));
        files.push((path, mesh.bytes));
    }
    files.push((
        directory.join("preview.json"),
        scene_export::writer::flax::live_snapshot(&built.level, &built.collision).into_bytes(),
    ));
    Ok(Prepared {
        files,
        meshes,
        scene: format!("{FOLDER}/preview.json"),
        warnings: built.warnings,
    })
}

/// Use the complete export pipeline so imported models and authored materials travel too.
pub fn prepare_document(
    snapshot: crate::document::ExportSnapshot,
    options: &Options,
    root: &Path,
) -> Result<Prepared, String> {
    let document = crate::document::Document::from_export_snapshot(snapshot)?;
    let directory = root.join(FOLDER);
    let mut options = options.clone();
    options.name = "preview".into();
    options.directory = directory.to_string_lossy().into_owned();
    options.content_dir = "HeronBrushLive/preview".into();
    let (report, exported) =
        crate::export::export_files(&document, &directory, &options).map_err(|e| e.to_string())?;
    let mut metadata = None;
    let mut environment = None;
    let mut replacements = std::collections::BTreeMap::new();
    let mut files = Vec::new();
    let mut meshes = Vec::new();
    for (path, contents) in exported {
        let bytes = match contents {
            crate::export::Contents::Bytes(bytes) => bytes,
            crate::export::Contents::Text(text) => text.into_bytes(),
        };
        if path.file_name().is_some_and(|n| n == "preview.level.json") {
            metadata = Some(
                serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|e| e.to_string())?,
            );
        } else if path
            .file_name()
            .is_some_and(|n| n == "preview.environment.json")
        {
            environment = Some(
                serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|e| e.to_string())?,
            );
        } else if path
            .extension()
            .is_some_and(|ext| ext == "glb" || ext == "dds")
        {
            let ext = path.extension().unwrap().to_string_lossy();
            let name = format!("{}.{}", fingerprint(&bytes), ext);
            let relative = format!("HeronBrushLive/preview/{name}");
            replacements.insert(
                path.file_name().unwrap().to_string_lossy().into_owned(),
                relative.clone(),
            );
            if ext == "glb" {
                meshes.push(format!("Content/{relative}"));
            }
            files.push((directory.join("preview").join(name), bytes));
        }
    }
    let mut metadata = metadata.ok_or("Flax export did not produce level metadata")?;
    for entity in metadata["entities"]
        .as_array_mut()
        .ok_or("Invalid Flax level metadata")?
    {
        if let Some(mesh) = entity["mesh"].as_str() {
            let name = mesh.rsplit('/').next().unwrap_or(mesh);
            entity["mesh"] =
                serde_json::json!(replacements.get(name).ok_or("Missing exported Flax mesh")?);
        }
    }
    let mut snapshot = scene_export::writer::flax::live_snapshot_metadata(&metadata)?;
    if let Some(mut env) = environment {
        if env["sky"].is_string() {
            env["sky"] =
                serde_json::json!(replacements.get("sky.dds").ok_or("Missing Flax sky cube")?);
        }
        if let Some(probes) = env["probes"].as_array_mut() {
            for probe in probes {
                let name = probe["texture"].as_str().ok_or("Invalid probe texture")?;
                probe["texture"] =
                    serde_json::json!(replacements.get(name).ok_or("Missing Flax probe cube")?);
            }
        }
        snapshot["environment"] = env;
    }
    files.push((
        directory.join("preview.json"),
        serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?,
    ));
    Ok(Prepared {
        files,
        meshes,
        scene: format!("{FOLDER}/preview.json"),
        warnings: report.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "writes a native Flax lighting fixture to HERON_FLAX_ADVANCED"]
    fn native_flax_advanced_fixture() {
        let root = PathBuf::from(std::env::var_os("HERON_FLAX_ADVANCED").unwrap());
        std::fs::create_dir_all(&root).unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        fn copy_tree(source: &Path, destination: &Path) {
            std::fs::create_dir_all(destination).unwrap();
            for entry in source.read_dir().unwrap() {
                let entry = entry.unwrap();
                let target = destination.join(entry.file_name());
                if entry.path().is_dir() {
                    copy_tree(&entry.path(), &target);
                } else {
                    std::fs::copy(entry.path(), target).unwrap();
                }
            }
        }
        copy_tree(
            &repository.join("demos/feature_lab/textures"),
            &root.join("textures"),
        );
        let mut document = crate::document::Document::open(
            repository.join("demos/feature_lab/feature_lab.map"),
            Default::default(),
        )
        .unwrap();
        document
            .load_materials(repository.join("demos/feature_lab/textures"))
            .unwrap();
        document.set_entity_property_on(
            &[crate::document::Document::WORLDSPAWN],
            "fog_enabled",
            "1",
        );
        document.set_entity_property_on(
            &[crate::document::Document::WORLDSPAWN],
            "fog_density",
            "0.02",
        );
        crate::light_probes::save(
            &mut document,
            &crate::light_probes::Settings {
                resolution: 16,
                probes: vec![crate::light_probes::Probe {
                    position: [0., 0., 128.],
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let camera = crate::render_camera::Settings {
            exposure: 1.4,
            shadows: false,
            ..Default::default()
        };
        crate::render_camera::save(&mut document, Default::default(), &camera);
        let options = Options {
            target: crate::export::Target::Flax,
            name: "feature_lab".into(),
            project_dir: root.to_string_lossy().into_owned(),
            texture_dir: "textures".into(),
            entities: serde_json::from_value(serde_json::json!({"fl_probe":{"defaults":{"charge":9,"armed":true,"sequence":[2,3,5],"label":"probe"}}})).unwrap(),
            ..Default::default()
        };
        crate::export::export(&document, &root, &options).unwrap();
    }
    #[test]
    fn complete_snapshot_keeps_imported_models_and_immutable_environment() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let map = repository.join("demos/feature_lab/feature_lab.map");
        let document = crate::document::Document::open(&map, Default::default()).unwrap();
        let root = std::env::temp_dir().join(format!("heron-flax-complete-{}", std::process::id()));
        let options = Options {
            target: crate::export::Target::Flax,
            ..Default::default()
        };
        let prepared = prepare_document(document.export_snapshot(&map), &options, &root).unwrap();
        assert!(prepared
            .files
            .iter()
            .all(|(p, _)| p.extension().is_none_or(|e| e != "cs")));
        let (_, bytes) = prepared
            .files
            .iter()
            .find(|(p, _)| p.ends_with("preview.json"))
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let placements = snapshot["placements"].as_array().unwrap();
        assert!(placements
            .iter()
            .any(|p| p["name"] == "heron_model" && p["mesh"].is_string()));
        for p in placements {
            if let Some(mesh) = p["mesh"].as_str() {
                assert!(mesh.starts_with("HeronBrushLive/preview/"));
                assert!(prepared
                    .files
                    .iter()
                    .any(|(path, _)| path == &root.join("Content").join(format!("{mesh}.glb"))));
            }
        }
        let sky = snapshot["environment"]["sky"].as_str().unwrap();
        assert!(sky.starts_with("HeronBrushLive/preview/"));
        assert!(prepared
            .files
            .iter()
            .any(|(p, _)| p == &root.join("Content").join(sky)));
        assert!(snapshot["environment"]["display"].is_object());
        let options = Options {
            sky: false,
            ..options
        };
        let without_sky =
            prepare_document(document.export_snapshot(&map), &options, &root).unwrap();
        let (_, bytes) = without_sky
            .files
            .iter()
            .find(|(p, _)| p.ends_with("preview.json"))
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        assert!(snapshot["environment"]["sky"].is_null());
        assert!(snapshot["environment"]["display"].is_object());
    }
    #[test]
    fn addon_updates_owned_sources() {
        let root = std::env::temp_dir().join(format!("heron-flax-upgrade-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Source/Game")).unwrap();
        std::fs::write(root.join("Source/Game/Game.Build.cs"), "test module").unwrap();
        install(&root).unwrap();
        let folder = module(&root).unwrap();
        let name = "HeronBrushLiveEditor.cs";
        let old = b"previous generated addon";
        std::fs::write(folder.join(name), old).unwrap();
        let mut owned: std::collections::BTreeMap<String, String> =
            serde_json::from_slice(&std::fs::read(folder.join(OWNERSHIP)).unwrap()).unwrap();
        owned.insert(name.into(), fingerprint(old));
        std::fs::write(folder.join(OWNERSHIP), serde_json::to_vec(&owned).unwrap()).unwrap();
        assert!(!installed(&root));
        install(&root).unwrap();
        assert!(installed(&root));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn addon_install_is_repeatable_and_preserves_edits() {
        let root = std::env::temp_dir().join(format!("heron-flax-install-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Source/Game")).unwrap();
        std::fs::write(root.join("Source/Game/Game.Build.cs"), "test module").unwrap();
        install(&root).unwrap();
        install(&root).unwrap();
        assert!(installed(&root));
        let path = module(&root).unwrap().join("HeronBrushLiveEditor.cs");
        std::fs::write(&path, "local edits").unwrap();
        assert!(!installed(&root));
        assert!(install(&root).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "local edits");
        std::fs::remove_dir_all(root).unwrap();
    }
}
