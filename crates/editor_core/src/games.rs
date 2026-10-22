//! The games the editor knows about, and what each one brings with it.
//!
//! A game directory contains `GameConfig.cfg`, which names map formats, entity definitions,
//! material paths and tool materials. Configurations are discovered in the `games` folder of the
//! profile directory, in `games` beside the application, and (when running from a checkout)
//! in the repository's `gamedata`.
//!
//! [`scene_export::gameconfig`] reads game configurations as JSON with comments and optional trailing
//! commas.
//!
//! What a game is *for* here is opening a map: it says which format to write, which FGD to
//! load, and where the materials are, none of which the editor could otherwise know. The
//! game path -- the directory holding the actual assets -- is the user's, since a config
//! shipped with a game says nothing about where that user keeps it.

use std::path::{Path, PathBuf};

use crate::preferences;

/// A game the editor can open a map for.
#[derive(Debug, Clone)]
pub struct Game {
    /// What the config calls it.
    pub name: String,
    /// The directory the config was read from, which is where its FGDs live.
    pub directory: PathBuf,
    /// Icon named by the configuration, relative to its directory.
    pub icon: Option<PathBuf>,
    /// Base-game directory below the installation root.
    pub search_path: Option<String>,
    /// The map formats it accepts, in the order the config lists them; the first is what
    /// a new map is written in, and each may name a map to start from.
    pub formats: Vec<Format>,
    /// The entity definitions to load, as paths under the config's own directory.
    pub definitions: Vec<PathBuf>,
    /// Where its materials live, under the game path.
    pub materials_root: Option<String>,
    /// Game-relative indexed texture palette.
    pub materials_palette: Option<String>,
    /// The material a clip brush wears, when the config names one.
    pub clip_material: Option<String>,
    /// The material a skipped face wears, when the config names one.
    pub skip_material: Option<String>,
    /// The tags the configuration gives brushes, in the order it lists them.
    pub brush_tags: Vec<Tag>,
    /// The tags it gives brush faces.
    pub face_tags: Vec<Tag>,
    /// `faceattribs.surfaceflags` names, in bit order -- entry `i` names the bit `1 << i`,
    /// an unused slot an empty string. What `FaceAttribsEditor` labels its ticks with.
    pub surface_flag_names: Vec<String>,
    /// `faceattribs.contentflags` names, in bit order.
    pub content_flag_names: Vec<String>,
}

/// A name a configuration gives to the brushes or faces that match a rule.
///
/// Game configuration tags classify brushes by classname, material or surface/content flags. They
/// drive visibility filters and selection actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// What the configuration calls it, which is what the view filter lists.
    pub name: String,
    /// What decides whether it applies.
    pub rule: TagRule,
    /// The glob the rule is tested against.
    pub pattern: String,
    /// Whether the configuration marks it as something drawn through.
    pub transparent: bool,
}

/// What a tag matches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagRule {
    /// The classname of the entity the brush belongs to.
    Classname,
    /// The material a face wears.
    Material,
    /// A bit of the face's content flags.
    ContentFlag,
    /// A bit of its surface flags.
    SurfaceFlag,
    /// A `surfaceparm` of the shader the face's material came from,
    /// `SurfaceParmTagMatcher`.
    SurfaceParm,
    /// Something this does not test, so the tag never applies.
    Unknown,
}

impl Tag {
    /// Whether a name matches this tag's pattern.
    ///
    /// Material patterns use case-insensitive glob matching.
    pub fn matches_name(&self, name: &str) -> bool {
        text_utils::string_compare::str_matches_glob(
            name,
            &self.pattern,
            text_utils::string_compare::CaseSensitivity::Insensitive,
        )
    }

    /// Whether a flags value carries any bit this tag names -- `FlagsTagMatcher::matches`,
    /// which is an overlap test, not a subset one, so "Liquid" (lava|slime|water) matches
    /// a face that is only water.
    pub fn matches_flags(&self, flags: i32) -> bool {
        self.pattern
            .trim()
            .parse::<i32>()
            .is_ok_and(|mask| mask != 0 && flags & mask != 0)
    }
}

/// The tags one block of a configuration lists.
fn read_tags(tags: &[scene_export::gameconfig::FaceTag]) -> Vec<Tag> {
    tags.iter()
        .map(|tag| Tag {
            name: tag.name.clone(),
            rule: match tag.match_kind.as_str() {
                "classname" => TagRule::Classname,
                "material" | "texture" => TagRule::Material,
                "contentflag" => TagRule::ContentFlag,
                "surfaceflag" => TagRule::SurfaceFlag,
                "surfaceparm" => TagRule::SurfaceParm,
                _ => TagRule::Unknown,
            },
            pattern: tag.pattern.clone(),
            transparent: tag.attribs.iter().any(|attrib| attrib == "transparent"),
        })
        .collect()
}

/// One of the formats a game accepts, and what a new map in it starts from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    /// The dialect the map is written in.
    pub format: map_model::map_format::MapFormat,
    /// What the configuration calls it, which is what the chooser shows -- a game may
    /// list two spellings of the same dialect, and the name is how they are told apart.
    pub name: String,
    /// A map to open a new document from, under the configuration's own directory.
    ///
    /// The configured `initialmap` is copied into a new document as its starting geometry.
    pub initial_map: Option<PathBuf>,
}

impl Game {
    /// User-facing name for the bundled general-purpose profile.
    pub fn display_name(&self) -> &str {
        if self.name == "heron_brush" {
            "Heron Brush"
        } else {
            &self.name
        }
    }

    /// Ordered roots: configured base game, installation root, then mods separated by `;`.
    /// Later roots override earlier roots, including their loose files.
    pub fn asset_roots(&self, game_path: &Path, mods: &str) -> Vec<PathBuf> {
        if game_path.as_os_str().is_empty() {
            return Vec::new();
        }
        let mut roots = Vec::new();
        if let Some(base) = self
            .search_path
            .as_deref()
            .filter(|base| !base.is_empty() && *base != ".")
        {
            if !game_path.ends_with(base) {
                roots.push(game_path.join(base));
            }
        }
        roots.push(game_path.to_path_buf());
        for name in mods
            .split(';')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            let path = game_path.join(name);
            roots.retain(|root| root != &path);
            roots.push(path);
        }
        roots
    }

    /// The FGD to load: the first the config names that is actually there.
    pub fn definition_file(&self) -> Option<PathBuf> {
        self.definitions
            .iter()
            .find(|path| crate::files::is_file(path))
            .cloned()
    }

    /// Where this game's materials are, given where the user keeps the game itself.
    pub fn materials_directory(&self, game_path: &Path) -> Option<PathBuf> {
        let root = self.materials_root.as_ref()?;
        let directory = game_path.join(root);
        crate::files::is_dir(&directory).then_some(directory)
    }
}

/// The directories a game configuration is looked for in.
///
/// [`user_games_directory`] holds a user's own; `games` beside the executable is where a
/// package puts the ones it ships, and a build run from the checkout reads the
/// repository's `gamedata` instead. All are read, and a user's own wins where the names
/// collide, since that is the one they edited.
fn search_paths() -> Vec<PathBuf> {
    let mut paths = vec![user_games_directory()];
    let beside = crate::preferences::install_directory().map(|dir| dir.join("games"));
    match beside.filter(|dir| crate::files::is_dir(dir)) {
        Some(beside) => paths.push(beside),
        None => paths.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gamedata")),
    }
    paths
}

/// Where a user's own game configurations go: `games` in the profile directory, one
/// folder per game, each holding its `GameConfig.cfg`.
pub fn user_games_directory() -> PathBuf {
    crate::preferences::config_dir().join("games")
}

/// Every game found, sorted by name.
///
/// A config that cannot be read is left out rather than reported: the list is what the
/// editor can offer, and a broken config offers nothing. `read` gives the error when one
/// game is asked for by name.
pub fn all() -> Vec<Game> {
    let mut found: Vec<Game> = Vec::new();
    for search in search_paths() {
        let Ok(entries) = crate::files::read_dir(&search) else {
            continue;
        };
        for entry in entries {
            let config = entry.join("GameConfig.cfg");
            if !crate::files::is_file(&config) {
                continue;
            }
            let Ok(game) = read(&config) else {
                continue;
            };
            // Heron Brush is our general-purpose profile; do not offer the upstream duplicate.
            if game.name == "Generic" {
                continue;
            }
            if !found.iter().any(|already| already.name == game.name) {
                found.push(game);
            }
        }
    }
    found.sort_by(|one, other| {
        (one.name != "heron_brush")
            .cmp(&(other.name != "heron_brush"))
            .then_with(|| one.display_name().cmp(other.display_name()))
    });
    found
}

/// Reads one game configuration.
pub fn read(config: &Path) -> Result<Game, scene_export::gameconfig::GameConfigError> {
    let read = scene_export::gameconfig::parse_file(config)?;
    let directory = config
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    Ok(Game {
        name: if read.name.is_empty() {
            directory
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "game".to_string())
        } else {
            read.name.clone()
        },
        search_path: read.search_path.clone(),
        icon: read.icon.as_ref().map(|name| directory.join(name)),
        formats: read_formats(config),
        definitions: read
            .definitions
            .iter()
            .map(|name| directory.join(name))
            .collect(),
        materials_root: read.materials_root.clone(),
        materials_palette: read.materials_palette.clone(),
        clip_material: read.tool_texture("clip"),
        skip_material: read.tool_texture("skip"),
        brush_tags: read_tags(&read.brush_tags),
        face_tags: read_tags(&read.face_tags),
        surface_flag_names: read.surface_flags.clone(),
        content_flag_names: read.content_flags.clone(),
        directory,
    })
}

/// The map formats a config lists, in order, with whatever each starts a new map from.
///
/// `scene_export::gameconfig` has no use for these and does not keep them, so they are read here.
/// The shape is fixed -- `{ "format": "Valve", "initialmap": "initial_valve.map" }` -- so
/// they are found by name rather than by parsing the JSON a second time; the file is JSON
/// with comments, which is why the parser it goes through in the first place is not
/// `serde_json` on its own.
///
/// A config that names none, or only ones this editor does not write, is taken to mean the
/// Valve dialect, which is what everything modern uses.
fn read_formats(config: &Path) -> Vec<Format> {
    let Ok(text) = crate::files::read_to_string(config) else {
        return vec![valve_format()];
    };
    let directory = config.parent().unwrap_or(Path::new("."));

    let mut formats: Vec<Format> = Vec::new();
    for piece in text.split("\"format\"").skip(1) {
        let Some(name) = piece.split('"').nth(1) else {
            continue;
        };
        let format = match name {
            spelling if spelling.eq_ignore_ascii_case("Standard") => {
                map_model::map_format::MapFormat::Standard
            }
            // Every other spelling a game uses -- "Valve", "Quake2 (Valve)", "Quake3
            // (Valve)" -- writes the Valve dialect's face lines, which is the part of the
            // format this editor decides. What the surface attributes carry comes from
            // the map itself.
            spelling if spelling.to_ascii_lowercase().contains("valve") => {
                map_model::map_format::MapFormat::Valve
            }
            _ => continue,
        };

        // The map to start from, if this entry names one before the next entry begins.
        let entry = piece.split('{').next().unwrap_or(piece);
        let initial_map = entry
            .split("\"initialmap\"")
            .nth(1)
            .and_then(|rest| rest.split('"').nth(1))
            .map(|name| directory.join(name));

        if !formats.iter().any(|already| already.name == name) {
            formats.push(Format {
                format,
                name: name.to_string(),
                initial_map,
            });
        }
    }

    if formats.is_empty() {
        formats.push(valve_format());
    }
    formats
}

/// The Valve dialect, for a configuration that names no format this editor can write.
fn valve_format() -> Format {
    Format {
        format: map_model::map_format::MapFormat::Valve,
        name: "Valve".to_string(),
        initial_map: None,
    }
}

/// The game the preferences say to start from, if it is still there.
pub fn current() -> Option<Game> {
    let wanted = preferences::pref(|p| p.game.clone());
    if wanted.is_empty() {
        return None;
    }
    all().into_iter().find(|game| game.name == wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `GameConfig.cfg` in its usual shape, comments and all.
    const CONFIG: &str = r#"
// A game configuration.
{
    "version": 9,
    "name": "example",
    "icon": "badge.png",
    "fileformats": [
        { "format": "Valve", "initialmap": "initial_valve.map" },
        { "format": "Standard" }
    ],
    "materials": { "root": "textures", "extensions": [".png"] },
    "entities": { "definitions": [ "example.fgd" ] },
    "tags": {
        "brush": [],
        "brushface": [
            { "name": "Clip", "match": "material", "pattern": "clip", "attribs": [] },
            { "name": "Skip", "match": "material", "pattern": "skip", "attribs": [] },
            { "name": "Detail", "match": "contentflag", "flags": ["detail"] }
        ]
    },
    "faceattribs": {
        "surfaceflags": [ { "name": "light" }, { "name": "slick" } ],
        "contentflags": [ { "name": "solid" }, { "unused": true }, { "name": "detail" } ]
    }
}
"#;

    fn written(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("heron-games-{name}"));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory is made");
        let config = directory.join("GameConfig.cfg");
        std::fs::write(&config, CONFIG).expect("the config is written");
        config
    }

    #[test]
    fn every_bundled_profile_and_definition_is_readable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gamedata");
        let expected = [
            ("heron_brush", "heron_brush"),
            ("DDayNormandy", "D-Day Normandy"),
            ("Daikatana", "Daikatana"),
            ("DigitalPaintball2", "Digital Paint: Paintball 2"),
            ("Generic", "Generic"),
            ("Halflife", "Half-Life"),
            ("Heretic2", "Heretic 2"),
            ("Hexen2", "Hexen 2"),
            ("Kingpin", "Kingpin"),
            ("Neverball", "Neverball"),
            ("Quake", "Quake"),
            ("Quake2", "Quake 2"),
            ("Quake3", "Quake 3"),
            ("Quetoo", "Quetoo"),
            ("SoF", "SoF"),
            ("Wrath", "Wrath"),
        ];
        for (folder, name) in expected {
            let game = read(&root.join(folder).join("GameConfig.cfg"))
                .unwrap_or_else(|error| panic!("{folder}: {error}"));
            assert_eq!(game.name, name);
            assert!(
                game.icon.as_ref().is_some_and(|path| path.is_file()),
                "{folder} icon"
            );
            assert!(!game.formats.is_empty(), "{folder}");
            assert!(game.definition_file().is_some(), "{folder}");
            for path in &game.definitions {
                let source = std::fs::read_to_string(path).unwrap();
                let definitions = match path.extension().and_then(|ext| ext.to_str()) {
                    Some("def") => map_model::def::read_def(&source).unwrap(),
                    Some("ent") => map_model::ent::read_ent(&source).unwrap(),
                    _ => map_model::fgd::read_fgd_file(path).unwrap(),
                };
                assert!(
                    definitions.placeable().next().is_some(),
                    "{}",
                    path.display()
                );
            }
            if folder == "heron_brush" {
                assert_eq!(game.display_name(), "Heron Brush");
            }
        }
    }

    #[test]
    fn game_roots_follow_the_profile_then_explicit_mod_order() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gamedata");
        let quake = read(&root.join("Quake/GameConfig.cfg")).unwrap();
        let install = std::env::temp_dir().join("heron-game-roots");
        assert_eq!(
            quake.asset_roots(&install, "mod_a; mod_b;mod_a"),
            vec![
                install.join("id1"),
                install.clone(),
                install.join("mod_b"),
                install.join("mod_a")
            ]
        );
        assert_eq!(
            quake.asset_roots(&install.join("id1"), ""),
            vec![install.join("id1")]
        );
        assert!(quake.asset_roots(Path::new(""), "mod_a").is_empty());
    }

    #[test]
    fn a_config_gives_its_name_formats_and_definitions() {
        let config = written("read");
        let game = read(&config).expect("it reads");

        assert_eq!(game.name, "example");
        assert_eq!(game.icon, Some(config.parent().unwrap().join("badge.png")));
        let formats: Vec<map_model::map_format::MapFormat> =
            game.formats.iter().map(|format| format.format).collect();
        assert_eq!(
            formats,
            vec![
                map_model::map_format::MapFormat::Valve,
                map_model::map_format::MapFormat::Standard
            ],
            "in the order the config lists them"
        );
        assert!(
            game.formats[0]
                .initial_map
                .as_ref()
                .is_some_and(|path| path.ends_with("initial_valve.map")),
            "the first format starts a new map from the one it names"
        );
        assert_eq!(game.formats[1].initial_map, None);
        assert_eq!(game.definitions.len(), 1);
        assert!(game.definitions[0].ends_with("example.fgd"));
        assert_eq!(game.materials_root.as_deref(), Some("textures"));
        assert_eq!(game.clip_material.as_deref(), Some("clip"));
        assert_eq!(game.skip_material.as_deref(), Some("skip"));

        // The faceattribs flag names carry through in bit order, unused slots and all.
        assert_eq!(game.surface_flag_names, vec!["light", "slick"]);
        assert_eq!(game.content_flag_names, vec!["solid", "", "detail"]);
        // And a flag tag's names became the bitmask.
        let detail = game.face_tags.iter().find(|t| t.name == "Detail").unwrap();
        assert!(detail.matches_flags(1 << 2));

        let _ = std::fs::remove_dir_all(config.parent().expect("it has a directory"));
    }

    #[test]
    fn a_definition_that_is_not_there_is_not_offered() {
        let config = written("definitions");
        let game = read(&config).expect("it reads");
        assert_eq!(game.definition_file(), None, "the fgd was never written");

        std::fs::write(&game.definitions[0], "@PointClass = info_player_start []\n")
            .expect("the fgd is written");
        assert_eq!(game.definition_file(), Some(game.definitions[0].clone()));

        let _ = std::fs::remove_dir_all(config.parent().expect("it has a directory"));
    }
}
