//! Flax live snapshots and installation of the editor-side scene importer.
use crate::{
    export::Options,
    godot_live::{Prepared, atomic_write},
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
        ("HeronBrushLevelRoot.cs",include_bytes!("../../scene_export/src/writer/flax_level_root.cs").to_vec()),
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
pub fn installed(root: &Path) -> bool {
    module(root).is_ok_and(|p| addon().iter().all(|(name, _)| p.join(name).is_file()))
}
pub fn install(root: &Path) -> Result<(), String> {
    let module = module(root)?;
    let files = addon();
    for (name, bytes) in &files {
        let path = module.join(name);
        if path.exists() && std::fs::read(&path).map_err(|e| e.to_string())? != *bytes {
            return Err(format!(
                "{} has local edits. Move it aside before installing the addon.",
                path.display()
            ));
        }
    }
    for (name, bytes) in files {
        atomic_write(&module.join(name), &bytes).map_err(|e| e.to_string())?;
    }
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
    let classes=crate::export_bindings::classes(&map,classes,&options.entities)?;
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

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(install(&root).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "local edits");
        std::fs::remove_dir_all(root).unwrap();
    }
}
