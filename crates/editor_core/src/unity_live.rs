//! Unity live snapshots; the shared publisher supplies debounce, worker and commit protocol.
use crate::{
    export::Options,
    godot_live::{atomic_write, Prepared},
};
use std::path::{Path, PathBuf};
const ADDON: &[(&str, &[u8])] = &[
    (
        "Assets/Editor/HeronBrushLive/HeronBrushLiveWindow.cs",
        include_bytes!("unity_live/live_window.cs"),
    ),
    (
        "Assets/HeronBrushLiveSupport/HeronBrushLiveRoot.cs",
        include_bytes!("unity_live/live_root.cs"),
    ),
    (
        "Assets/HeronBrushLiveSupport/HeronBrushLiveEntity.cs",
        include_bytes!("unity_live/live_entity.cs"),
    ),
];
fn addon() -> Vec<(&'static str, Vec<u8>)> {
    ADDON
        .iter()
        .map(|(path, bytes)| {
            (
                *path,
                String::from_utf8_lossy(bytes)
                    .replace(
                        "        // HERON_COLLISION_HELPER",
                        scene_export::writer::unity::COLLISION_HELPER,
                    )
                    .replace(
                        "        // HERON_BINDINGS_HELPER",
                        include_str!("../../scene_export/src/writer/unity_bindings.cs"),
                    )
                    .into_bytes(),
            )
        })
        .collect()
}
pub fn project(options: &Options) -> Result<PathBuf, String> {
    if options.directory.is_empty() {
        return Err("Export this map into a Unity project's Assets folder first.".into());
    }
    let dir = Path::new(&options.directory)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let root = if options.project_dir.is_empty() {
        dir.ancestors()
            .find(|p| p.join("ProjectSettings/ProjectVersion.txt").is_file())
            .map(Path::to_path_buf)
            .ok_or("The export folder must be inside a Unity project.")?
    } else {
        Path::new(&options.project_dir)
            .canonicalize()
            .map_err(|e| e.to_string())?
    };
    let assets = root
        .join("Assets")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !root.join("ProjectSettings/ProjectVersion.txt").is_file() || !dir.starts_with(assets) {
        return Err("Choose an export folder inside the Unity project's Assets directory.".into());
    }
    Ok(root)
}
pub fn installed(root: &Path) -> bool {
    ADDON.iter().all(|(path, _)| root.join(path).is_file())
}
pub fn install(root: &Path) -> Result<(), String> {
    for (path, bytes) in addon() {
        let path = root.join(path);
        if path.exists() && std::fs::read(&path).map_err(|e| e.to_string())? != bytes {
            return Err(format!(
                "{} has local edits. Move it aside before installing this addon.",
                path.display()
            ));
        }
    }
    for (path, bytes) in addon() {
        atomic_write(&root.join(path), &bytes).map_err(|e| e.to_string())?;
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
    let map = scene_export::map::parse(text, &options.name).map_err(|e| e.to_string())?;
    let classes = crate::export_bindings::classes(&map, classes, &options.entities)?;
    let mut built = scene_export::builder::build_level(
        &map,
        &classes,
        settings,
        &scene_export::builder::LevelOptions {
            name: options.name.clone(),
            mesh_dir: "meshes".into(),
            collision: options.collision,
            inline_collision: true,
            mesh_collision: false,
            convex_collision: false,
            mesh_format: scene_export::builder::MeshFormat::Obj,
        },
    );
    scene_export::writer::unity::validate(&built.level)?;
    let directory = Path::new(&options.directory);
    let mut files = vec![];
    let mut meshes = vec![];
    let mtl = crate::export::paint_mtl(settings, &format!("../{}_paint", options.name));
    let mtl_name = format!(
        "{}.mtl",
        crate::export::paint_export::image_name(mtl.as_bytes())
    );
    if !settings.generated_materials.is_empty() {
        let path = directory.join("meshes").join(&mtl_name);
        meshes.push(
            path.strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/"),
        );
        files.push((path, mtl.into_bytes()));
    }
    for mesh in &built.meshes {
        let grouped = String::from_utf8_lossy(&mesh.bytes)
            .replace("\no ", "\ng ")
            .into_bytes();
        let bytes = if settings.generated_materials.is_empty() {
            grouped.clone()
        } else {
            let mut bytes = format!("mtllib {mtl_name}\n").into_bytes();
            bytes.extend_from_slice(&grouped);
            bytes
        };
        // Immutable mesh names prevent Unity auto-refresh from changing the old branch mid-batch.
        let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        });
        let name = format!("{hash:016x}_{}", mesh.file_name);
        for entity in &mut built.level.entities {
            if entity.mesh.as_deref() == Some(&format!("meshes/{}", mesh.file_name)) {
                entity.mesh = Some(format!("meshes/{name}"));
            }
        }
        let path = directory.join("meshes").join(name);
        meshes.push(
            path.strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/"),
        );
        files.push((path, bytes));
    }
    let path = directory.join("live.json");
    let scene = path
        .strip_prefix(root)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .replace('\\', "/");
    files.push((
        path,
        scene_export::writer::unity::live_data(&built.level, &built.collision).into_bytes(),
    ));
    Ok(Prepared {
        files,
        meshes,
        scene,
        warnings: built.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document::Document, export::Target, world::WorldBounds};
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    struct Temp(PathBuf);
    impl Temp {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("heron-unity-live-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("Assets/Maps")).unwrap();
            std::fs::create_dir_all(path.join("ProjectSettings")).unwrap();
            std::fs::write(
                path.join("ProjectSettings/ProjectVersion.txt"),
                "m_EditorVersion: 6000.0.0f1",
            )
            .unwrap();
            Self(path)
        }
        fn options(&self) -> Options {
            Options {
                target: Target::Unity,
                directory: self.0.join("Assets/Maps").to_string_lossy().into(),
                name: "live".into(),
                cull_hidden: false,
                merge_faces: false,
                ..Default::default()
            }
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn unity_project_requires_assets_and_install_preserves_edits() {
        let root = Temp::new("install");
        let mut options = root.options();
        assert_eq!(project(&options).unwrap(), root.0.canonicalize().unwrap());
        options.directory = root.0.to_string_lossy().into();
        assert!(project(&options).is_err());
        install(&root.0).unwrap();
        install(&root.0).unwrap();
        assert!(installed(&root.0));
        let path = root.0.join(ADDON[0].0);
        std::fs::write(&path, "local edits").unwrap();
        assert!(install(&root.0).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "local edits");
    }
    #[test]
    fn snapshots_cover_unsaved_geometry_deletion_undo_and_typed_lights_without_scripts() {
        let root = Temp::new("snapshots");
        let mut options = root.options();
        options.directory = root
            .0
            .join("Assets/HeronBrushLive")
            .to_string_lossy()
            .into();
        let mut doc = Document::new(WorldBounds::default());
        let brush = map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &Bbox3d::from_scalars(-32., 32.),
            "__TB_empty",
        )
        .unwrap();
        doc.add_brushes(vec![brush]);
        let (classes, _) = crate::export::entity_classes(&doc);
        let settings = options.settings(Path::new(&options.directory));
        let snapshot = |doc: &Document| {
            prepare(&doc.to_map_text(), &classes, &settings, &options, &root.0).unwrap()
        };
        let first = snapshot(&doc);
        assert_eq!(first.meshes.len(), 1);
        assert!(first
            .files
            .iter()
            .all(|(p, _)| p.extension().unwrap() != "cs"));
        let read = |p: &Prepared| -> serde_json::Value {
            serde_json::from_slice(
                &p.files
                    .iter()
                    .find(|(p, _)| p.ends_with("live.json"))
                    .unwrap()
                    .1,
            )
            .unwrap()
        };
        assert!(!read(&first)["entities"][0]["shapes"]
            .as_array()
            .unwrap()
            .is_empty());
        doc.translate_selection(Vec3d::new(128., 0., 0.));
        let moved = snapshot(&doc);
        assert_ne!(
            moved.meshes, first.meshes,
            "geometry updates must not mutate assets used by the previous snapshot"
        );
        doc.undo();
        assert_eq!(snapshot(&doc).meshes, first.meshes);
        doc.delete_selection();
        let deleted = snapshot(&doc);
        assert!(deleted.meshes.is_empty());
        doc.undo();
        assert_eq!(snapshot(&doc).meshes, first.meshes);
        doc.create_point_entity("light_spot", Vec3d::new(32., 64., 96.));
        let data = read(&snapshot(&doc));
        let light = data["entities"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["light"].is_object())
            .unwrap();
        assert_eq!(light["light"]["kind"], "Spot");
        assert_eq!(light["light"]["cone"], 90.);
        assert!(light["metadata"].as_str().unwrap().contains("light_spot"));
    }
}
