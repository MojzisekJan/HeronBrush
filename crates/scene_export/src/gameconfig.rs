//! Reader for `GameConfig.cfg` game configurations, which is JSON with comments and
//! the odd trailing comma, so it is cleaned up before `serde_json` sees it.

use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum GameConfigError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} is not valid JSON: {source}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone)]
pub struct FaceTag {
    pub name: String,
    /// What the tag matches on: `material`, `contentflag`, `surfaceflag`.
    pub match_kind: String,
    /// For a `material` tag, the material-name glob. For a `contentflag` / `surfaceflag`
    /// tag, the bitmask of the flags it names, written in decimal -- resolved here from
    /// the `flags` array against `faceattribs`'s ordered flag list.
    pub pattern: String,
    pub attribs: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct GameConfig {
    pub path: Option<PathBuf>,
    pub version: i64,
    pub name: String,
    pub icon: Option<String>,
    pub search_path: Option<String>,
    /// Directory holding textures, relative to the game's search path.
    pub materials_root: Option<String>,
    pub materials_extensions: Vec<String>,
    /// `materials.excludes`: globs naming the images that are not materials of their own,
    /// such as Quetoo's `*_norm` and `*_spec` maps.
    pub materials_excludes: Vec<String>,
    /// External palette resource used by indexed textures.
    pub materials_palette: Option<String>,
    /// `materials.attribute`: the worldspawn key that lists the map's texture packages,
    /// `wad` for the Quake lineage.
    pub materials_attribute: Option<String>,
    /// FGD or DEF files listed under `entities.definitions`.
    pub definitions: Vec<String>,
    /// `entities.scale`. The format allows an expression; a plain number is
    /// the only form that carries over to the build.
    pub entity_scale: Option<f64>,
    pub brush_tags: Vec<FaceTag>,
    pub face_tags: Vec<FaceTag>,
    pub default_uv_scale: Option<(f64, f64)>,
    /// `faceattribs.surfaceflags`, in bit order -- entry `i` is the flag `1 << i`. An
    /// unused slot is an empty string.
    pub surface_flags: Vec<String>,
    /// `faceattribs.contentflags`, in bit order.
    pub content_flags: Vec<String>,
    /// `heron.mergeGroupsOnExport`, a key of Heron's own: the game's compiler does not
    /// read `func_group`, so an exported map has its groups and layers merged into the
    /// worldspawn. TrenchBroom's copies of the profiles do not have it.
    pub merge_groups_on_export: bool,
    /// `heron.builtinEntities`, a key of Heron's own: whether the entity browser offers
    /// Heron's built-in classes beside the profile's. Unset leaves the choice to the
    /// editor.
    pub builtin_entities: Option<bool>,
    /// `heron.searchPathAlternatives`, a key of Heron's own: folders a game that shares
    /// the profile keeps its base data in instead of `filesystem.searchpath`, such as
    /// OpenArena's `baseoa` for Quake 3's `baseq3`.
    pub search_path_alternatives: Vec<String>,
}

impl GameConfig {
    /// The texture named by the face tag called `name`, matched case-insensitively.
    pub fn tool_texture(&self, name: &str) -> Option<String> {
        self.face_tags
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name) && t.match_kind == "material")
            .map(|t| t.pattern.clone())
    }

    /// Directory the config lives in, which is treated as the game root.
    pub fn root_dir(&self) -> Option<&Path> {
        self.path.as_deref().and_then(|p| p.parent())
    }
}

pub fn parse_file(path: &Path) -> Result<GameConfig, GameConfigError> {
    let text = file_store::read_to_string(path).map_err(|source| GameConfigError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let mut config = parse(&text, &path.display().to_string())?;
    config.path = Some(path.to_path_buf());
    Ok(config)
}

pub fn parse(text: &str, path: &str) -> Result<GameConfig, GameConfigError> {
    let cleaned = strip_comments_and_trailing_commas(text);
    let root: Value = serde_json::from_str(&cleaned).map_err(|source| GameConfigError::Json {
        path: path.to_string(),
        source,
    })?;

    let mut config = GameConfig {
        search_path: root
            .pointer("/filesystem/searchpath")
            .and_then(Value::as_str)
            .map(str::to_owned),
        icon: root.get("icon").and_then(Value::as_str).map(str::to_string),
        version: root.get("version").and_then(Value::as_i64).unwrap_or(0),
        name: root
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        ..Default::default()
    };

    // Version 9 renamed "textures" to "materials" and flattened the root path.
    let materials = root.get("materials").or_else(|| root.get("textures"));
    if let Some(m) = materials {
        config.materials_palette = m.get("palette").and_then(Value::as_str).map(str::to_string);
        config.materials_attribute = m
            .get("attribute")
            .and_then(Value::as_str)
            .map(str::to_string);
        config.materials_root = m
            .get("root")
            .and_then(Value::as_str)
            .or_else(|| m.pointer("/package/root").and_then(Value::as_str))
            .map(str::to_string);
        let exts = m
            .get("extensions")
            .or_else(|| m.pointer("/format/extensions"));
        if let Some(Value::Array(list)) = exts {
            config.materials_extensions = list
                .iter()
                .filter_map(Value::as_str)
                .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
                .collect();
        }
        if let Some(Value::Array(list)) = m.get("excludes") {
            config.materials_excludes = list
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
        }
    }

    if let Some(entities) = root.get("entities") {
        if let Some(Value::Array(defs)) = entities.get("definitions") {
            config.definitions = defs
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
        }
        config.entity_scale = match entities.get("scale") {
            Some(Value::Number(n)) => n.as_f64(),
            Some(Value::String(s)) => s.trim().parse().ok(),
            _ => None,
        };
    }

    config.merge_groups_on_export = root
        .pointer("/heron/mergeGroupsOnExport")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    config.builtin_entities = root
        .pointer("/heron/builtinEntities")
        .and_then(Value::as_bool);
    config.search_path_alternatives = root
        .pointer("/heron/searchPathAlternatives")
        .and_then(Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    config.surface_flags = parse_flag_names(root.pointer("/faceattribs/surfaceflags"));
    config.content_flags = parse_flag_names(root.pointer("/faceattribs/contentflags"));

    if let Some(tags) = root.get("tags") {
        config.brush_tags = parse_tags(
            tags.get("brush"),
            &config.surface_flags,
            &config.content_flags,
        );
        config.face_tags = parse_tags(
            tags.get("brushface"),
            &config.surface_flags,
            &config.content_flags,
        );
    }

    if let Some(scale) = root.pointer("/faceattribs/defaults/scale") {
        if let Some(list) = scale.as_array() {
            let nums: Vec<f64> = list.iter().filter_map(Value::as_f64).collect();
            if nums.len() >= 2 {
                config.default_uv_scale = Some((nums[0], nums[1]));
            }
        }
    }

    Ok(config)
}

/// The ordered flag names of a `faceattribs.surfaceflags` / `contentflags` array: entry
/// `i` names the flag `1 << i`, and an `{ "unused": true }` slot is an empty string.
fn parse_flag_names(value: Option<&Value>) -> Vec<String> {
    let Some(Value::Array(list)) = value else {
        return Vec::new();
    };
    list.iter()
        .map(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

/// The bitmask of the named flags, `1 << i` for each name at index `i` of `flag_names`.
fn flag_mask(names: &[&str], flag_names: &[String]) -> i32 {
    let mut mask = 0i32;
    for name in names {
        if let Some(bit) = flag_names.iter().position(|f| f.eq_ignore_ascii_case(name)) {
            if bit < 32 {
                mask |= 1 << bit;
            }
        }
    }
    mask
}

fn parse_tags(
    value: Option<&Value>,
    surface_flags: &[String],
    content_flags: &[String],
) -> Vec<FaceTag> {
    let Some(Value::Array(list)) = value else {
        return Vec::new();
    };
    list.iter()
        .map(|t| {
            let match_kind = t
                .get("match")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();

            // A flag tag names its flags; resolve them to the mask `matches_flags` wants.
            let pattern = match match_kind.as_str() {
                "contentflag" | "surfaceflag" => {
                    let flag_names = if match_kind == "surfaceflag" {
                        surface_flags
                    } else {
                        content_flags
                    };
                    let named: Vec<&str> = t
                        .get("flags")
                        .and_then(Value::as_array)
                        .map(|a| a.iter().filter_map(Value::as_str).collect())
                        .unwrap_or_default();
                    flag_mask(&named, flag_names).to_string()
                }
                _ => t
                    .get("pattern")
                    .and_then(Value::as_str)
                    .or_else(|| t.get("material").and_then(Value::as_str))
                    .unwrap_or_default()
                    .to_string(),
            };

            FaceTag {
                name: t
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                match_kind,
                pattern,
                attribs: t
                    .get("attribs")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect()
}

/// Removes `//` and `/* */` comments and commas that sit before a closing
/// bracket, leaving valid JSON. Comment markers inside strings are left alone.
fn strip_comments_and_trailing_commas(text: &str) -> String {
    let bytes: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    let mut in_string = false;

    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            out.push(c);
            if c == '\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == '/' {
            while i < bytes.len() && bytes[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == '*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == '*' && bytes[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        out.push(c);
        i += 1;
    }

    // Second pass for trailing commas, now that comments are gone.
    let chars: Vec<char> = out.chars().collect();
    let mut result = String::with_capacity(chars.len());
    let mut in_string = false;
    for (i, c) in chars.iter().enumerate() {
        if in_string {
            result.push(*c);
            if *c == '"' && chars.get(i.wrapping_sub(1)) != Some(&'\\') {
                in_string = false;
            }
            continue;
        }
        if *c == '"' {
            in_string = true;
            result.push(*c);
            continue;
        }
        if *c == ',' {
            let next = chars[i + 1..].iter().find(|c| !c.is_whitespace());
            if matches!(next, Some(']') | Some('}')) {
                continue;
            }
        }
        result.push(*c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: &str = r#"
// heron_brush game configuration
{
    "version": 9,
    "name": "heron_brush",
    "icon": "Icon.png",
    "fileformats": [ { "format": "Valve", "initialmap": "initial_valve.map" } ],
    "filesystem": { "searchpath": ".", "packageformat": { "extension": ".zip", "format": "zip" } },
    "materials": {
        "root": "textures",
        "extensions": [".png", ".jpg"],
        "attribute": "wad",
        "palette": "pics/colormap.pcx",
        "excludes": [ "*_norm", "*_glow" ],
    },
    "entities": {
        "definitions": [ "heron_brush.fgd" ],
        "defaultcolor": "0.6 0.6 0.6 1.0",
        "scale": "32"
    },
    "tags": {
        "brush": [ { "name": "Trigger", "attribs": [ "transparent" ], "match": "classname", "pattern": "trigger_*" } ],
        "brushface": [
            { "name": "Clip", "attribs": [ "transparent" ], "match": "material", "pattern": "clip" },
            { "name": "Skip", "attribs": [ "transparent" ], "match": "material", "pattern": "skip" },
            { "name": "Origin", "attribs": [ "transparent" ], "match": "material", "pattern": "origin" }
        ]
    },
    "faceattribs": { "defaults": { "scale": [0.5, 0.5] }, "contentflags": [], "surfaceflags": [] }
}
"#;

    #[test]
    fn reads_a_version_9_config() {
        let cfg = parse(CFG, "GameConfig.cfg").unwrap();
        assert_eq!(cfg.version, 9);
        assert_eq!(cfg.name, "heron_brush");
        assert_eq!(cfg.materials_root.as_deref(), Some("textures"));
        assert_eq!(cfg.materials_extensions, vec!["png", "jpg"]);
        assert_eq!(cfg.materials_palette.as_deref(), Some("pics/colormap.pcx"));
        assert_eq!(cfg.materials_attribute.as_deref(), Some("wad"));
        assert_eq!(cfg.materials_excludes, vec!["*_norm", "*_glow"]);
        assert_eq!(cfg.definitions, vec!["heron_brush.fgd"]);
        assert_eq!(cfg.icon.as_deref(), Some("Icon.png"));
        assert_eq!(cfg.search_path.as_deref(), Some("."));
        assert_eq!(cfg.entity_scale, Some(32.0));
        assert_eq!(cfg.tool_texture("clip").as_deref(), Some("clip"));
        assert_eq!(cfg.tool_texture("origin").as_deref(), Some("origin"));
        assert_eq!(cfg.default_uv_scale, Some((0.5, 0.5)));
        assert_eq!(cfg.brush_tags.len(), 1);
        assert!(!cfg.merge_groups_on_export);
    }

    #[test]
    fn reads_whether_an_export_merges_groups() {
        let cfg = parse(
            r#"{ "version": 9, "name": "Neverball",
                 // Heron's own section.
                 "heron": { "mergeGroupsOnExport": true } }"#,
            "GameConfig.cfg",
        )
        .expect("the config parses");
        assert!(cfg.merge_groups_on_export);
        assert_eq!(cfg.builtin_entities, None);
    }

    #[test]
    fn reads_whether_the_builtin_entities_are_offered() {
        let cfg = parse(
            r#"{ "version": 9, "name": "Mine", "heron": { "builtinEntities": true } }"#,
            "GameConfig.cfg",
        )
        .expect("the config parses");
        assert_eq!(cfg.builtin_entities, Some(true));
        assert!(cfg.search_path_alternatives.is_empty());
    }

    #[test]
    fn reads_the_search_path_alternatives() {
        let cfg = parse(
            r#"{ "version": 9, "name": "Quake 3",
                 "heron": { "searchPathAlternatives": [ "baseoa", 3 ] } }"#,
            "GameConfig.cfg",
        )
        .expect("the config parses");
        assert_eq!(cfg.search_path_alternatives, vec!["baseoa"]);
    }

    #[test]
    fn a_flag_tag_resolves_its_flag_names_to_a_bitmask() {
        let cfg = parse(
            r#"{
                "version": 9, "name": "q2",
                "tags": { "brushface": [
                    { "name": "Detail", "match": "contentflag", "flags": ["detail"] },
                    { "name": "Liquid", "match": "contentflag", "flags": ["lava", "slime", "water"] },
                    { "name": "Hint",   "match": "surfaceflag", "flags": ["hint"] }
                ] },
                "faceattribs": {
                    "surfaceflags": [ { "name": "light" }, { "name": "slick" }, { "name": "sky" }, { "name": "warp" }, { "name": "trans33" }, { "name": "trans66" }, { "name": "flowing" }, { "name": "nodraw" }, { "name": "hint" } ],
                    "contentflags": [ { "name": "solid" }, { "name": "window" }, { "name": "aux" }, { "name": "lava" }, { "name": "slime" }, { "name": "water" }, { "name": "mist" }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "unused": true }, { "name": "detail" } ]
                }
            }"#,
            "GameConfig.cfg",
        )
        .unwrap();

        let by_name = |name: &str| {
            cfg.face_tags
                .iter()
                .find(|t| t.name == name)
                .unwrap()
                .pattern
                .clone()
        };
        // `detail` is content bit 27.
        assert_eq!(by_name("Detail"), (1 << 27).to_string());
        // lava|slime|water are bits 3, 4, 5.
        assert_eq!(
            by_name("Liquid"),
            ((1 << 3) | (1 << 4) | (1 << 5)).to_string()
        );
        // `hint` is surface bit 8.
        assert_eq!(by_name("Hint"), (1 << 8).to_string());
    }

    #[test]
    fn reads_the_older_version_4_layout() {
        let cfg = parse(
            r#"{ "version": 4, "name": "old",
                 "textures": { "package": { "type": "directory", "root": "tex" },
                               "format": { "extensions": ["png"], "format": "image" } } }"#,
            "GameConfig.cfg",
        )
        .unwrap();
        assert_eq!(cfg.materials_root.as_deref(), Some("tex"));
        assert_eq!(cfg.materials_extensions, vec!["png"]);
    }

    #[test]
    fn comment_markers_inside_strings_survive() {
        let cfg = parse(r#"{ "name": "a // b", "version": 9 }"#, "c").unwrap();
        assert_eq!(cfg.name, "a // b");
    }
}
