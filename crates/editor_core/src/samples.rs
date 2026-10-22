//! The example maps bundled with the editor.
//!
//! Bundled samples are embedded in the binary and written to the configuration directory when first
//! needed. Their generator scripts use the MCP document API.

use std::path::{Path, PathBuf};
#[path = "motion_sample_resources.rs"]
mod motion_resources;
#[path = "sample_resources.rs"]
mod resources;

/// One bundled example.
pub struct Sample {
    /// The file it is written as; its stem is what the welcome window shows.
    pub file: &'static str,
    /// The `.map` text.
    pub map: &'static str,
}

/// Every bundled example, in the order the welcome window lists them.
pub const SAMPLES: &[Sample] = &[
    Sample {
        file: "motion_lab/motion_lab.map",
        map: include_str!("../../../demos/motion_lab/motion_lab.map"),
    },
    Sample {
        file: "water_lab/water_lab.map",
        map: include_str!("../../../demos/water_lab/water_lab.map"),
    },
    Sample {
        file: "lantern_lane/lantern_lane.map",
        map: include_str!("../../../demos/lantern_lane/lantern_lane.map"),
    },
    Sample {
        file: "dragon.map",
        map: include_str!("../../../assets/dragon.map"),
    },
    Sample {
        file: "bookshelf.map",
        map: include_str!("../../../assets/bookshelf.map"),
    },
    Sample {
        file: "bridge.map",
        map: include_str!("../../../assets/bridge.map"),
    },
    Sample {
        file: "airship.map",
        map: include_str!("../../../assets/airship.map"),
    },
    Sample {
        file: "boat.map",
        map: include_str!("../../../assets/boat.map"),
    },
    Sample {
        file: "plane.map",
        map: include_str!("../../../assets/plane.map"),
    },
    Sample {
        file: "windmill.map",
        map: include_str!("../../../assets/windmill.map"),
    },
    Sample {
        file: "locomotive.map",
        map: include_str!("../../../assets/locomotive.map"),
    },
    Sample {
        file: "tram.map",
        map: include_str!("../../../assets/tram.map"),
    },
    Sample {
        file: "truck.map",
        map: include_str!("../../../assets/truck.map"),
    },
    Sample {
        file: "bench.map",
        map: include_str!("../../../assets/bench.map"),
    },
    Sample {
        file: "crate.map",
        map: include_str!("../../../assets/crate.map"),
    },
    Sample {
        file: "lamp.map",
        map: include_str!("../../../assets/lamp.map"),
    },
    Sample {
        file: "pillar.map",
        map: include_str!("../../../assets/pillar.map"),
    },
];

/// Where the examples are written, beside the preferences.
pub fn directory() -> PathBuf {
    crate::preferences::config_dir().join("examples")
}

/// Writes any example that is not already on disk (or whose text has changed) and
/// returns every example's path, in listing order. A path that could not be written is
/// left out rather than handed back broken.
pub fn ensure() -> Vec<PathBuf> {
    // The web build has no disk to write them to: their paths are handed out as they are,
    // and opening one reads the bundled text ([`bundled`]).
    if cfg!(target_arch = "wasm32") {
        return SAMPLES
            .iter()
            .map(|sample| directory().join(sample.file))
            .collect();
    }
    ensure_in(&directory())
}

/// The bundled text of the example at `path`, where [`ensure`] would have written it.
pub fn bundled(path: &Path) -> Option<&'static str> {
    let file = path.strip_prefix(directory()).ok()?;
    SAMPLES
        .iter()
        .find(|sample| Path::new(sample.file) == file)
        .map(|sample| sample.map)
}

fn ensure_in(dir: &Path) -> Vec<PathBuf> {
    let _ = std::fs::create_dir_all(dir);
    for (name, bytes) in resources::FILES.iter().chain(motion_resources::FILES) {
        let path = dir.join(name);
        // Motion lab is a working project: preserve any edits in the user copy.
        if name.starts_with("motion_lab/") && path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::read(&path).ok().as_deref() != Some(*bytes) {
            let _ = std::fs::write(&path, bytes);
        }
    }
    let mut paths = Vec::with_capacity(SAMPLES.len());
    for sample in SAMPLES {
        let path = dir.join(sample.file);
        if sample.file.starts_with("motion_lab/") && path.is_file() {
            paths.push(path);
            continue;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let up_to_date = std::fs::read_to_string(&path)
            .map(|on_disk| on_disk == sample.map)
            .unwrap_or(false);
        if !up_to_date && std::fs::write(&path, sample.map).is_err() {
            continue;
        }
        paths.push(path);
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_map_parses() {
        let bounds = geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0);
        for sample in SAMPLES {
            let map = map_model::io::map_reader::read_map(sample.map, None)
                .unwrap_or_else(|e| panic!("{} does not parse: {e}", sample.file));
            let tree = map_model::node::build_tree(&map, &bounds);
            assert!(
                !map_model::node::brushes(&tree.world).is_empty(),
                "{} has no brushes",
                sample.file
            );
        }
    }

    #[test]
    fn canal_demo_opens_with_bundled_textures_and_no_dropped_brushes() {
        let dir = std::env::temp_dir().join(format!("heron-demo-resources-{}", std::process::id()));
        ensure_in(&dir);
        let path = dir.join("lantern_lane/lantern_lane.map");
        let text = std::fs::read_to_string(&path).unwrap();
        let parsed = map_model::io::map_reader::read_map(&text, None).unwrap();
        let bounds = geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0);
        let tree = map_model::node::build_tree(&parsed, &bounds);
        let mut document =
            crate::document::Document::open(&path, crate::world::WorldBounds::default()).unwrap();
        document
            .load_definitions(dir.join("lantern_lane/lantern_lane.fgd"))
            .unwrap();
        // Keyframe and water entities are demonstrated by their dedicated lab maps.
        let demo_text = format!(
            "{}\n{}",
            text,
            include_str!("../../../demos/motion_lab/motion_lab.map")
        ) + include_str!("../../../demos/water_lab/water_lab.map");
        for definition in document.definitions().unwrap().placeable() {
            assert!(
                demo_text.contains(&format!("\"classname\" \"{}\"", definition.name)),
                "missing demo entity {}",
                definition.name
            );
        }
        let source_brushes: usize = parsed.entities.iter().map(|e| e.brushes.len()).sum();
        assert!(source_brushes > 2000);
        assert_eq!(document.brush_count(), source_brushes);
        assert_eq!(map_model::node::brushes(&tree.world).len(), source_brushes);
        for brush in map_model::node::brushes(&tree.world) {
            for face in brush.faces() {
                if face.material_name.starts_with("kenney_fantasy/") {
                    assert!(
                        document.materials().get(&face.material_name).is_some(),
                        "{}",
                        face.material_name
                    );
                }
            }
        }
        assert!(dir.join("lantern_lane/sounds/well.wav").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn motion_lab_is_portable_and_preserves_user_edits() {
        let dir =
            std::env::temp_dir().join(format!("heron-motion-resources-{}", std::process::id()));
        ensure_in(&dir);
        let map = dir.join("motion_lab/motion_lab.map");
        let doc = crate::document::Document::open(&map, Default::default()).unwrap();
        assert!(doc.brush_count() > 20);
        for file in [
            "godot/project.godot",
            "godot/maps/motion_lab.tscn",
            "godot/player.gd",
            "godot/verify.gd",
            "motion_lab.fgd",
        ] {
            assert!(dir.join("motion_lab").join(file).is_file(), "{file}");
        }
        std::fs::write(&map, "user map changes").unwrap();
        let player = dir.join("motion_lab/godot/player.gd");
        std::fs::write(&player, "user controller changes").unwrap();
        ensure_in(&dir);
        assert_eq!(std::fs::read_to_string(map).unwrap(), "user map changes");
        assert_eq!(
            std::fs::read_to_string(player).unwrap(),
            "user controller changes"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ensure_writes_then_leaves_alone() {
        let dir = std::env::temp_dir().join("heron-samples-test");
        let _ = std::fs::remove_dir_all(&dir);

        let first = ensure_in(&dir);
        assert_eq!(first.len(), SAMPLES.len());
        for path in &first {
            assert!(path.is_file());
        }

        // A second call rewrites nothing: the mtimes stay put.
        let before: Vec<_> = first
            .iter()
            .map(|p| std::fs::metadata(p).unwrap().modified().unwrap())
            .collect();
        std::thread::sleep(web_time::Duration::from_millis(10));
        let second = ensure_in(&dir);
        let after: Vec<_> = second
            .iter()
            .map(|p| std::fs::metadata(p).unwrap().modified().unwrap())
            .collect();
        assert_eq!(before, after);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
