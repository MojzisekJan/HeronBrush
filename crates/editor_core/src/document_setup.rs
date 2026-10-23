//! Map, entity-definition and material loading before views share the document.

use crate::{document::Document, games, world::WorldBounds};
use std::path::PathBuf;

/// Where the definitions and materials in use were read from, so they can be read again.
///
/// `Menu/File/Reload Entity Definitions` and `Reload Material Collections`: a `.fgd` or a
/// texture folder is edited while the editor is open, and F5 or F6 picks the change up
/// without reopening the map.
#[derive(Default)]
pub struct Sources {
    /// The entity definitions in use.
    pub definitions: Option<std::path::PathBuf>,
    /// The material folder in use.
    pub materials: Option<std::path::PathBuf>,
}

/// A ready document and the resources its reload commands should use.
pub struct LoadedDocument {
    pub document: Document,
    pub sources: Sources,
}

pub fn load(
    path: Option<PathBuf>,
    fgd: Option<PathBuf>,
    materials: Option<PathBuf>,
) -> LoadedDocument {
    // The world size belongs to the target game, not to the editor; this is the default
    // until there is a game configuration to read it from.
    let bounds = WorldBounds::default();

    let mut document = match &path {
        Some(path) => match Document::open(path, bounds) {
            Ok(document) => {
                report_warnings(&document);
                document
            }
            Err(error) => {
                crate::complain!("{}: {error}", path.display());
                Document::new(bounds)
            }
        },
        // Loads the configured initial map as a template and clears its path so the first save asks
        // for a new destination.
        None => match games::current()
            .and_then(|game| game.formats.first().and_then(|f| f.initial_map.clone()))
            .filter(|template| crate::files::is_file(template))
        {
            Some(template) => match Document::open(&template, bounds) {
                Ok(mut document) => {
                    crate::say!("started from {}", template.display());
                    document.path = None;
                    document
                }
                Err(error) => {
                    crate::complain!("{}: {error}", template.display());
                    Document::new(bounds)
                }
            },
            None => Document::new(bounds),
        },
    };

    // Loads entity definitions and materials from the selected game. Explicit command-line files
    // take precedence.
    let game = games::current();
    if let Some(game) = &game {
        crate::say!("game: {}", game.name);
    }
    apply_game(&mut document, game.as_ref(), path.is_some());
    let fgd = fgd.or_else(|| definition_file(path.as_deref(), game.as_ref()));
    let materials = materials.or_else(|| document.project_material_folder());

    // What was loaded, so F5 and F6 can load it again after it has been edited.
    let mut sources = Sources::default();

    // Entity definitions are optional: without them a map still opens, the editor just
    // cannot say what an entity's properties mean.
    if let Some(fgd) = fgd {
        match document.load_definitions(&fgd) {
            Ok(warnings) => {
                crate::say!("loaded entity definitions from {}", fgd.display());
                sources.definitions = Some(fgd.clone());
                for warning in warnings {
                    crate::complain!("warning: {warning}");
                }
            }
            Err(error) => crate::complain!("{}: {error}", fgd.display()),
        }
    }

    // Loads texture archives listed in the map worldspawn `wad` property.
    {
        for (wad, loaded) in document.load_named_wads() {
            match loaded {
                Ok(warnings) => {
                    crate::say!("loaded textures from {}", wad.display());
                    for warning in warnings {
                        crate::complain!("wad: {warning}");
                    }
                }
                Err(error) => crate::complain!("{}: {error}", wad.display()),
            }
        }
    }

    if let Some(materials) = &materials {
        match document.load_materials(materials) {
            Ok(warnings) => {
                sources.materials = Some(materials.clone());
                for warning in warnings {
                    crate::complain!("materials: {warning}");
                }
            }
            Err(error) => crate::complain!("{}: {error}", materials.display()),
        }
    }

    crate::assets::load_library_materials(&mut document);
    LoadedDocument { document, sources }
}

/// Gives a document what its game profile says about maps: the tags the view panel
/// offers to hide, the names of the face flags and, for a new map, its dialect.
///
/// Prints nothing, since the MCP server calls it too and its standard output is the
/// protocol.
pub fn apply_game(document: &mut Document, game: Option<&games::Game>, opened: bool) {
    document.set_offer_builtin_entities(game.is_none_or(|game| game.builtin_entities));
    let Some(game) = game else {
        return;
    };
    document.set_tags(game.brush_tags.clone(), game.face_tags.clone());
    document.set_face_flag_names(
        game.surface_flag_names.clone(),
        game.content_flag_names.clone(),
    );
    document.set_merge_groups_on_export(game.merge_groups_on_export);
    document.set_writes_default_properties(game.set_default_properties);
    document.set_default_uv_scale(game.default_uv_scale);
    // A map that was opened keeps whatever it was read as, since that is what it
    // is. A new one is the dialect that holds everything -- it used to take the
    // first format the game listed, which for a Quake-lineage game is the Standard
    // one, and that quietly decided at the moment of creation that no face in this
    // map may ever carry its own UV axes. Nothing about a new map should have to be
    // chosen before there is a map, and the flavour a level is *written* for belongs
    // to the export, not to the hours of editing before it.
    if !opened {
        document.format = map_model::map_format::MapFormat::LOSSLESS;
    }
}

/// The entity definitions a map should use: the one `.fgd` beside it, or else the game's.
pub fn definition_file(
    map: Option<&std::path::Path>,
    game: Option<&games::Game>,
) -> Option<PathBuf> {
    map.and_then(sibling_fgd)
        .or_else(|| game.and_then(|game| game.definition_file()))
}

/// Finds portable project textures beside a map or beside its enclosing `maps` directory.
/// Local textures take precedence; unrelated ancestor directories are not searched.
pub fn project_material_directory(map: &std::path::Path) -> Option<PathBuf> {
    let directory = map.parent()?;
    let local = directory.join("textures");
    if crate::files::is_dir(&local) {
        return Some(local);
    }
    let maps = directory.ancestors().find(|dir| {
        dir.file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("maps"))
    })?;
    let textures = maps.parent()?.join("textures");
    crate::files::is_dir(&textures).then_some(textures)
}

/// Returns the FGD file beside the given map, if there is exactly one.
///
/// Guessing is only helpful when it cannot be wrong, so a directory with several is left
/// for the user to choose from.
fn sibling_fgd(map: &std::path::Path) -> Option<std::path::PathBuf> {
    let directory = map.parent()?;
    let mut found: Vec<std::path::PathBuf> = crate::files::read_dir(directory)
        .ok()?
        .into_iter()
        .filter(|path| {
            path.extension().is_some_and(|extension| {
                extension.eq_ignore_ascii_case("fgd")
                    || extension.eq_ignore_ascii_case("def")
                    || extension.eq_ignore_ascii_case("ent")
            })
        })
        .collect();
    found.sort();

    (found.len() == 1).then(|| found.remove(0))
}

/// Prints whatever the reader had to work around, so a map that needed it is not silently
/// different from the file on disk.
pub fn report_warnings(document: &Document) {
    for warning in document.warnings() {
        crate::complain!("warning: {warning}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn new_default_map_has_no_issues_with_or_without_the_asset_library() {
        let previous = crate::preferences::pref(Clone::clone);
        for library_enabled in [true, false] {
            let mut preferences = crate::preferences::Preferences::default();
            if !library_enabled {
                preferences.asset_library.clear();
            }
            crate::preferences::set_current(preferences);
            let loaded = load(None, None, None);
            assert!(loaded.document.path.is_none());
            assert!(!loaded.document.is_modified());
            assert!(
                loaded.document.to_map_text().contains("info_player_start"),
                "must load the starting room"
            );
            let issues = crate::validation::validate(&loaded.document);
            assert!(issues.is_empty(), "new map issues: {issues:?}");
        }
        crate::preferences::set_current(previous);
    }

    #[test]
    fn opening_a_map_discovers_portable_project_textures_and_normals() {
        let root = Fixture(
            std::env::temp_dir().join(format!("heron-project-textures-{}", std::process::id())),
        );
        let maps = root.0.join("maps/district");
        let textures = root.0.join("textures/stone");
        std::fs::create_dir_all(&maps).unwrap();
        std::fs::create_dir_all(&textures).unwrap();
        let map = maps.join("tower.map");
        std::fs::write(&map, "{\n\"classname\" \"worldspawn\"\n}\n").unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([120, 110, 100, 255]))
            .save(textures.join("wall.png"))
            .unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([128, 128, 255, 255]))
            .save(textures.join("wall_normal.png"))
            .unwrap();
        let opened = Document::open(&map, WorldBounds::default()).unwrap();
        assert!(opened
            .materials()
            .get("stone/wall")
            .unwrap()
            .normal_map
            .is_some());
        assert!(!opened.is_modified());
        let saved = root.0.join("documents/saved.map");
        std::fs::create_dir_all(saved.parent().unwrap()).unwrap();
        let mut persisted = Document::open(&map, WorldBounds::default()).unwrap();
        persisted.save_as(&saved).unwrap();
        let text = std::fs::read_to_string(&saved).unwrap();
        assert!(text.contains("[\"folder\",\"../textures\"]"));
        assert!(!text.lines().next().unwrap().contains(".local"));
        let moved = root.0.with_extension("relocated");
        std::fs::rename(&root.0, &moved).unwrap();
        let reopened =
            Document::open(moved.join("documents/saved.map"), WorldBounds::default()).unwrap();
        assert!(reopened
            .materials()
            .get("stone/wall")
            .unwrap()
            .normal_map
            .is_some());
        std::fs::rename(&moved, &root.0).unwrap();
        let loaded = load(Some(map.clone()), None, None);
        assert_eq!(
            loaded.sources.materials.map(|p| p.canonicalize().unwrap()),
            Some(root.0.join("textures").canonicalize().unwrap())
        );
        let local = maps.join("textures");
        std::fs::create_dir_all(&local).unwrap();
        assert_eq!(project_material_directory(&map), Some(local));
        let unrelated = root.0.join("unrelated");
        std::fs::create_dir_all(&unrelated).unwrap();
        assert_eq!(project_material_directory(&unrelated.join("map.map")), None);
    }

    #[test]
    fn loading_preserves_resource_precedence_and_reports_only_loaded_sources() {
        let root = Fixture(
            std::env::temp_dir().join(format!("heron-document-setup-{}", std::process::id())),
        );
        let maps = root.0.join("maps");
        let textures = root.0.join("textures");
        std::fs::create_dir_all(&maps).unwrap();
        std::fs::create_dir_all(&textures).unwrap();
        let map = maps.join("room.map");
        std::fs::write(&map, "{\n\"classname\" \"worldspawn\"\n}\n").unwrap();
        let sibling = maps.join("room.fgd");
        let explicit = root.0.join("override.fgd");
        std::fs::write(&sibling, "@PointClass = sibling_entity : \"Sibling\" []").unwrap();
        std::fs::write(&explicit, "@PointClass = explicit_entity : \"Explicit\" []").unwrap();

        let inferred = load(Some(map.clone()), None, Some(textures.clone()));
        assert_eq!(inferred.document.path.as_ref(), Some(&map));
        assert_eq!(inferred.sources.definitions.as_ref(), Some(&sibling));
        assert_eq!(inferred.sources.materials.as_ref(), Some(&textures));
        assert!(inferred
            .document
            .definitions()
            .unwrap()
            .find("sibling_entity")
            .is_some());
        assert!(!inferred.document.is_modified());

        let overridden = load(
            Some(map.clone()),
            Some(explicit.clone()),
            Some(textures.clone()),
        );
        assert_eq!(overridden.sources.definitions.as_ref(), Some(&explicit));
        assert!(overridden
            .document
            .definitions()
            .unwrap()
            .find("explicit_entity")
            .is_some());
        assert!(overridden
            .document
            .definitions()
            .unwrap()
            .find("sibling_entity")
            .is_none());

        let failed = load(Some(map), Some(root.0.join("missing.fgd")), Some(textures));
        assert!(failed.sources.definitions.is_none());
        assert!(failed.document.path.is_some());
        assert!(!failed.document.is_modified());
    }
}
