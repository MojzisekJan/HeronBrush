//! What each engine's export is set to, kept between sessions.
//!
//! Each engine has a saved export profile with its unit scale, collision representation and
//! resource options.
//!
//! It is its own file rather than a preference each, as the recent documents are: six
//! engines with five settings apiece would be thirty keys in a store that is meant to be
//! read by a person, and none of them belongs in the preferences window.
//!
//! One line per engine, `engine=key=value key=value ...`, and an engine with no line
//! takes the defaults. A key the reader does not know is skipped and a key that is not
//! there keeps its default, so a file written by an older or newer build still reads.
//!
//! It began as a positional line -- five numbers and then a pipe-separated tail of paths
//! -- and outgrew it: each new setting was another field everything had to count past,
//! and the fourteenth was where that stopped being reasonable. The old shape is still
//! read, so nobody's settings are lost on the way across; it is never written again.
//! A line that cannot be read at all is skipped, since a setting worth losing is worth
//! less than the file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::export::{Options, Target};

/// Where the settings live, beside the preferences.
pub fn default_path() -> PathBuf {
    crate::preferences::config_dir().join("export-settings")
}

/// The nearest project profile, or a new profile at the nearest engine/project
/// root. Saved maps never inherit another project's last output directory.
pub fn project_path(map: &Path) -> PathBuf {
    let absolute = if map.is_absolute() {
        map.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(map)
    };
    let parent = absolute.parent().unwrap_or(Path::new("."));
    for folder in parent.ancestors() {
        let profile = folder.join(".heron-brush/exports.json");
        if crate::files::is_file(&profile) {
            return profile;
        }
    }
    for folder in parent.ancestors() {
        if [
            "project.godot",
            "game.project",
            "Cargo.toml",
            "package.json",
            "ProjectSettings/ProjectVersion.txt",
        ]
        .iter()
        .any(|name| crate::files::is_file(&folder.join(name)))
            || crate::files::read_dir(folder).ok().is_some_and(|entries| {
                entries.iter().any(|path| {
                    path.extension()
                        .is_some_and(|e| e == "flaxproj" || e == "sdpkg" || e == "sln")
                })
            })
        {
            return folder.join(".heron-brush/exports.json");
        }
    }
    parent.join(".heron-brush/exports.json")
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectProfile {
    version: u32,
    targets: std::collections::BTreeMap<String, serde_json::Value>,
}

/// The name an engine is written under, which is what its line begins with.
fn key(target: Target) -> &'static str {
    match target {
        Target::Godot => "godot",
        Target::Defold => "defold",
        Target::ThreeJs => "three",
        Target::Bevy => "bevy",
        Target::Raylib => "raylib",
        Target::Unity => "unity",
        Target::Unreal => "unreal",
        Target::Flax => "flax",
        Target::Stride => "stride",
        Target::Cocos => "cocos",
        Target::GDevelop => "gdevelop",
        Target::Obj => "obj",
        Target::Gltf => "gltf",
        Target::Glb => "glb",
    }
}

/// What every engine is set to.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// The engines that have been set to something other than their defaults.
    per_engine: HashMap<&'static str, Options>,
    error: Option<String>,
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        self.error.clone().map_or(Ok(()), Err)
    }
    pub fn for_map(map: Option<&Path>) -> Self {
        match map {
            Some(map) => Self::load_from(&project_path(map)),
            None => Self::load(),
        }
    }
    /// Reads the settings, or the defaults where there is nothing to read.
    pub fn load() -> Self {
        Self::load_from(&default_path())
    }

    /// Reads the settings from a given file, which is what the tests hand it: the
    /// settings a save re-exports from are read the same way the window writes them, so
    /// the two cannot drift apart in a test that only pokes at structs.
    pub fn load_from(store: &Path) -> Self {
        let mut settings = Settings::default();
        let text = match crate::files::read_to_string(store) {
            Ok(text) => text,
            Err(error) => {
                if error.kind() != std::io::ErrorKind::NotFound {
                    settings.error = Some(format!(
                        "cannot read export settings {}: {error}",
                        store.display()
                    ));
                }
                return settings;
            }
        };

        if store.file_name().is_some_and(|name| name == "exports.json") {
            let profile: ProjectProfile = match serde_json::from_str(&text) {
                Ok(profile) => profile,
                Err(error) => {
                    settings.error = Some(format!(
                        "invalid export profile {}: {error}",
                        store.display()
                    ));
                    return settings;
                }
            };
            if profile.version != 1 {
                settings.error = Some(format!(
                    "unsupported export profile version {}",
                    profile.version
                ));
                return settings;
            }
            let root = store
                .parent()
                .and_then(Path::parent)
                .unwrap_or(Path::new("."));
            for (name, overrides) in profile.targets {
                let Some(target) = Target::ALL.into_iter().find(|target| key(*target) == name)
                else {
                    settings.error = Some(format!("unknown export profile target {name}"));
                    return settings;
                };
                let mut merged =
                    serde_json::to_value(default_for(target)).expect("export defaults serialize");
                let Some(overrides) = overrides.as_object() else {
                    settings.error = Some(format!("{name} export profile must be an object"));
                    return settings;
                };
                for (key, value) in overrides {
                    merged[key] = value.clone();
                }
                let mut options: Options = match serde_json::from_value(merged) {
                    Ok(options) => options,
                    Err(error) => {
                        settings.error = Some(format!("invalid {name} export profile: {error}"));
                        return settings;
                    }
                };
                if !options.scale.is_finite()
                    || options.scale <= 0.
                    || options.trigger_layer > 32
                    || !options.lightmap_texel_size.is_finite()
                    || options.lightmap_texel_size <= 0.
                {
                    settings.error =
                        Some(format!("invalid numeric settings in {name} export profile"));
                    return settings;
                }
                for path in [&mut options.directory, &mut options.project_dir] {
                    if !path.is_empty() && Path::new(path).is_relative() {
                        *path = root.join(&*path).to_string_lossy().into_owned();
                    }
                }
                options.target = target;
                settings.per_engine.insert(key(target), options);
            }
            return settings;
        }

        for line in text.lines() {
            let Some((name, values)) = line.split_once('=') else {
                continue;
            };
            let Some(target) = Target::ALL
                .into_iter()
                .find(|target| key(*target) == name.trim())
            else {
                continue;
            };
            // A line is either the named shape or the positional one it grew out of.
            // The pipe is what says which: the old shape always had one, the new one
            // never does.
            let options = if values.contains('|') || !values.contains('=') {
                read_positional(target, values)
            } else {
                read_named(target, values)
            };
            if let Some(options) = options {
                settings.per_engine.insert(key(target), options);
            }
        }
        settings
    }

    /// What one engine is set to: what was read, or that engine's defaults.
    pub fn for_target(&self, target: Target) -> Options {
        let mut options = self
            .per_engine
            .get(key(target))
            .cloned()
            .unwrap_or_else(|| default_for(target));
        options.target = target;
        options
    }

    /// Sets what one engine is written with, and writes the file.
    pub fn set(&mut self, target: Target, options: Options) {
        self.set_in(target, options, &default_path());
    }

    /// [`Settings::set`], writing the file at `store` rather than the usual place.
    pub fn set_in(&mut self, target: Target, options: Options, store: &Path) {
        self.per_engine.insert(key(target), options);
        if let Err(why) = self.save(store) {
            crate::complain!("the export settings could not be written: {why}");
        }
    }

    /// Writes every engine's settings out to `path`.
    fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(directory) = path.parent() {
            crate::files::create_dir_all(directory)?;
        }
        if path.file_name().is_some_and(|name| name == "exports.json") {
            let root = path
                .parent()
                .and_then(Path::parent)
                .unwrap_or(Path::new("."));
            let mut targets = std::collections::BTreeMap::new();
            for target in Target::ALL {
                if !self.per_engine.contains_key(key(target)) {
                    continue;
                }
                let mut options = self.for_target(target);
                for value in [&mut options.directory, &mut options.project_dir] {
                    if let Ok(relative) = Path::new(value).strip_prefix(root) {
                        *value = if relative.as_os_str().is_empty() {
                            ".".into()
                        } else {
                            relative.to_string_lossy().replace('\\', "/")
                        };
                    }
                }
                targets.insert(
                    key(target).into(),
                    serde_json::to_value(options).map_err(std::io::Error::other)?,
                );
            }
            let bytes = serde_json::to_vec_pretty(&ProjectProfile {
                version: 1,
                targets,
            })
            .map_err(std::io::Error::other)?;
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::godot_live::atomic_write(path, &bytes)
            }
            #[cfg(target_arch = "wasm32")]
            {
                crate::files::write(path, bytes)
            }
        } else {
            crate::files::write(path, self.to_text().as_bytes())
        }
    }

    /// Every engine's settings as the text of the file.
    ///
    /// Split from the writing so a test can put a line through both halves without
    /// writing over the settings of whoever is running it.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        // In the order the chooser lists the engines, so the file reads like the window.
        for target in Target::ALL {
            let options = self.for_target(target);
            let switch = |on: bool| if on { "1" } else { "0" };
            // A value is split on whitespace when it is read back, so a path with a
            // space in it has to say so some other way.
            let text = |value: &str| value.replace(' ', "\\s");
            let mut line = format!("{}=", key(target));
            for (field, value) in [
                ("scale", options.scale.to_string()),
                ("collision", switch(options.collision).to_string()),
                ("cull", switch(options.cull_hidden).to_string()),
                ("merge_faces", switch(options.merge_faces).to_string()),
                ("static_cells", switch(options.static_cells).to_string()),
                (
                    "merge_collision",
                    switch(options.merge_collision).to_string(),
                ),
                ("sky", switch(options.sky).to_string()),
                ("textures", text(&options.texture_dir)),
                ("materials", text(&options.material_dir)),
                ("clip", text(&options.clip_texture)),
                ("skip", text(&options.skip_texture)),
                ("origin", text(&options.origin_texture)),
                ("assets", text(&options.asset_dir)),
                ("content", text(&options.content_dir)),
                ("model_material", text(&options.model_material)),
                ("model_sampler", text(&options.model_sampler)),
                ("project", text(&options.project_dir)),
                ("trigger_layer", options.trigger_layer.to_string()),
                ("lightmaps", switch(options.lightmap_uv2).to_string()),
                ("texel", options.lightmap_texel_size.to_string()),
                ("name_property", text(&options.name_property)),
                ("groups", switch(options.group_hierarchy).to_string()),
                ("flip_faces", switch(options.flip_faces).to_string()),
                ("flip_uv", switch(options.flip_uv_v).to_string()),
                ("out", text(&options.directory)),
                ("on_save", switch(options.export_on_save).to_string()),
            ] {
                // An empty string would read back as a key with no value; leaving the
                // key out says the same thing and keeps the line to what was set.
                if value.is_empty() {
                    continue;
                }
                line.push_str(&format!("{field}={value} "));
            }
            if !options.entities.is_empty() {
                let bytes =
                    serde_json::to_vec(&options.entities).expect("serializable entity rules");
                line.push_str("entities_hex=");
                for byte in bytes {
                    line.push_str(&format!("{byte:02x}"));
                }
                line.push(' ');
            }
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out
    }
}

/// Reads a line in the named shape, `key=value key=value`.
///
/// A value cannot hold a space, which is what lets the pairs be split on whitespace; the
/// only settings that could want one are directory paths, and one with a space in it is
/// written with the space escaped as `\s`.
fn read_named(target: Target, values: &str) -> Option<Options> {
    let mut options = default_for(target);
    options.target = target;
    for pair in values.split_whitespace() {
        let Some((field, value)) = pair.split_once('=') else {
            continue;
        };
        let text = || value.replace("\\s", " ");
        let on = || value == "1";
        match field {
            "scale" => options.scale = value.parse().ok()?,
            "collision" => options.collision = on(),
            "cull" => options.cull_hidden = on(),
            "merge_faces" => options.merge_faces = on(),
            "static_cells" => options.static_cells = on(),
            "entities_hex" => {
                if value.len() % 2 != 0 {
                    return None;
                }
                let bytes = value
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| {
                        std::str::from_utf8(pair)
                            .ok()
                            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                    })
                    .collect::<Option<Vec<_>>>()?;
                options.entities = serde_json::from_slice(&bytes).ok()?;
            }
            "merge_collision" => options.merge_collision = on(),
            "textures" => options.texture_dir = text(),
            "materials" => options.material_dir = text(),
            "clip" => options.clip_texture = text(),
            "skip" => options.skip_texture = text(),
            "origin" => options.origin_texture = text(),
            "assets" => options.asset_dir = text(),
            "content" => options.content_dir = text(),
            "model_material" => options.model_material = text(),
            "model_sampler" => options.model_sampler = text(),
            "project" => options.project_dir = text(),
            "trigger_layer" => options.trigger_layer = value.parse().unwrap_or(3),
            "lightmaps" => options.lightmap_uv2 = on(),
            "sky" => options.sky = on(),
            "texel" => options.lightmap_texel_size = value.parse().unwrap_or(0.2),
            "name_property" => options.name_property = text(),
            "groups" => options.group_hierarchy = on(),
            "flip_faces" => options.flip_faces = on(),
            "flip_uv" => options.flip_uv_v = on(),
            "out" => options.directory = text(),
            "on_save" => options.export_on_save = on(),
            // A key from a build that knows more than this one.
            _ => {}
        }
    }
    Some(options)
}

/// Reads the shape this file used to have: five numbers, then a pipe-separated tail of
/// strings that grew a field at a time. Read but never written.
fn read_positional(target: Target, values: &str) -> Option<Options> {
    let (numbers, tail) = values.split_once('|').unwrap_or((values, ""));
    let mut strings = tail.split('|');
    let mut string = || strings.next().unwrap_or("").trim().to_string();

    let mut values = numbers.split_whitespace();
    let mut number = || values.next().and_then(|word| word.parse::<f64>().ok());
    let scale = number()?;
    let mut switch = || values.next().map(|word| word == "1").unwrap_or(true);

    Some(Options {
        target,
        name: String::new(),
        scale,
        collision: switch(),
        cull_hidden: switch(),
        merge_faces: switch(),
        merge_collision: switch(),
        texture_dir: string(),
        material_dir: string(),
        clip_texture: string(),
        skip_texture: string(),
        origin_texture: string(),
        asset_dir: string(),
        directory: string(),
        // A line from before this field has nothing here, which reads as off -- an
        // export that starts writing itself unasked would be a surprise, so the absent
        // value is the safe one.
        export_on_save: string() == "1",
        // Everything past this point only ever existed in the named shape.
        ..default_for(target)
    })
}

/// What an engine starts at before anyone changes it.
///
/// The scale is the one that engine's own examples are built at: the Quake lineage's 32
/// units to a metre for Godot and Defold, whose importers expect it, and the same for the
/// rest since a level built at one scale should not change size by being written twice.
fn default_for(target: Target) -> Options {
    let mut options = Options {
        target,
        ..Options::default()
    };
    if matches!(target, Target::Gltf | Target::Glb) {
        options.collision = false;
        options.cull_hidden = false;
        options.merge_faces = false;
        options.merge_collision = false;
        options.lightmap_uv2 = false;
    }
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_profiles_relocate_paths_and_keep_projects_isolated() {
        let base =
            std::env::temp_dir().join(format!("heron-profile-relocation-{}", std::process::id()));
        let first = base.join("first");
        let second = base.join("second");
        std::fs::create_dir_all(first.join("maps/nested")).unwrap();
        std::fs::create_dir_all(second.join("maps")).unwrap();
        std::fs::write(first.join("project.godot"), "").unwrap();
        std::fs::write(second.join("project.godot"), "").unwrap();
        let map = first.join("maps/nested/arena.map");
        let path = project_path(&map);
        let mut settings = Settings::default();
        settings.set_in(
            Target::Godot,
            Options {
                target: Target::Godot,
                scale: 64.,
                directory: first.join("generated/maps").to_string_lossy().into_owned(),
                project_dir: first.to_string_lossy().into_owned(),
                export_on_save: true,
                ..Default::default()
            },
            &path,
        );
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains(first.to_str().unwrap()));
        assert_eq!(project_path(&map), path);
        std::fs::create_dir_all(second.join(".heron-brush")).unwrap();
        std::fs::copy(&path, second.join(".heron-brush/exports.json")).unwrap();
        let loaded = Settings::for_map(Some(&second.join("maps/arena.map")));
        loaded.validate().unwrap();
        let options = loaded.for_target(Target::Godot);
        assert_eq!(options.scale, 64.);
        assert!(options.directory.starts_with(second.to_str().unwrap()));
        assert!(options.export_on_save);
        assert!(!loaded.for_target(Target::Bevy).export_on_save);
        std::fs::write(
            second.join(".heron-brush/exports.json"),
            "{\"version\":999,\"targets\":{}}",
        )
        .unwrap();
        assert!(Settings::for_map(Some(&second.join("maps/arena.map")))
            .validate()
            .is_err());
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn an_engine_with_no_line_takes_the_defaults() {
        let settings = Settings::default();
        let godot = settings.for_target(Target::Godot);
        assert_eq!(godot.target, Target::Godot);
        assert_eq!(godot.scale, Options::default().scale);
    }

    #[test]
    fn what_was_written_for_one_engine_comes_back_for_that_engine() {
        let store = std::env::temp_dir().join("heron-export-settings-test");
        std::fs::write(
            &store,
            "godot=64 1 0 1 0|res://gfx|res://mats|noclip|nodraw||\nbevy=1 0 1 1 1\nnonsense=x\n",
        )
        .expect("the file is written");

        let settings = Settings::load_from(&store);
        let godot = settings.for_target(Target::Godot);
        assert_eq!(godot.scale, 64.0);
        assert!(godot.collision);
        assert!(!godot.cull_hidden);
        assert!(godot.merge_faces);
        assert!(!godot.merge_collision);
        assert_eq!(godot.texture_dir, "res://gfx");
        assert_eq!(godot.material_dir, "res://mats");
        assert_eq!(godot.clip_texture, "noclip");
        assert_eq!(godot.skip_texture, "nodraw");
        assert_eq!(godot.origin_texture, "");
        assert_eq!(godot.asset_dir, "");

        let bevy = settings.for_target(Target::Bevy);
        assert_eq!(bevy.scale, 1.0);
        assert!(!bevy.collision);

        // An engine the file says nothing about is still itself.
        let raylib = settings.for_target(Target::Raylib);
        assert_eq!(raylib.target, Target::Raylib);
        assert_eq!(raylib.scale, Options::default().scale);

        let _ = std::fs::remove_file(&store);
    }

    #[test]
    fn a_file_in_the_old_positional_shape_still_reads() {
        // What a build before the named shape wrote. Nobody's settings are lost on the
        // way across, which is the whole reason the old reader is still here.
        let store = std::env::temp_dir().join(format!("heron-settings-old-{}", std::process::id()));
        std::fs::write(
            &store,
            "godot=37 0 1 1 1|res://gfx||||||/home/somebody/game/maps|1\n",
        )
        .expect("the file is written");

        let godot = Settings::load_from(&store).for_target(Target::Godot);
        assert_eq!(godot.scale, 37.0);
        assert!(!godot.collision);
        assert_eq!(godot.texture_dir, "res://gfx");
        assert_eq!(godot.directory, "/home/somebody/game/maps");
        assert!(godot.export_on_save);
        // The settings that only ever existed in the named shape take their defaults.
        assert_eq!(godot.trigger_layer, Options::default().trigger_layer);
        assert!(godot.group_hierarchy);

        // And the oldest shape of all: five numbers and no tail, which is what a file
        // written before the paths existed looks like. It carries no pipe either, so
        // the reader has to tell it from the named shape by the absent `=` alone.
        std::fs::write(&store, "godot=37 0 1 1 1\ndefold=32 1 1 1 1\n").expect("written");
        let godot = Settings::load_from(&store).for_target(Target::Godot);
        assert_eq!(godot.scale, 37.0);
        assert!(!godot.collision);
        assert!(godot.texture_dir.is_empty());

        let _ = std::fs::remove_file(&store);
    }

    #[test]
    fn every_setting_survives_being_written_and_read_back() {
        let store =
            std::env::temp_dir().join(format!("heron-settings-named-{}", std::process::id()));
        let mut settings = Settings::default();
        let written = Options {
            entities: Default::default(),
            target: Target::Godot,
            name: String::new(),
            scale: 48.0,
            collision: false,
            sky: false,
            cull_hidden: false,
            merge_faces: false,
            static_cells: true,
            merge_collision: false,
            texture_dir: "res://gfx".to_string(),
            material_dir: "res://mats".to_string(),
            clip_texture: "noclip".to_string(),
            skip_texture: "nodraw".to_string(),
            origin_texture: "pivot".to_string(),
            asset_dir: "/main/arena".to_string(),
            content_dir: "/Game/My Levels".into(),
            model_material: "/main/custom.material".into(),
            model_sampler: "albedo".into(),
            project_dir: "/home/somebody/My Game".to_string(),
            trigger_layer: 7,
            lightmap_uv2: true,
            lightmap_texel_size: 0.35,
            name_property: "targetname".to_string(),
            group_hierarchy: false,
            flip_faces: true,
            flip_uv_v: true,
            // A path with a space in it, since values are split on whitespace.
            directory: "/home/somebody/My Game/maps".to_string(),
            export_on_save: true,
        };
        settings
            .per_engine
            .insert(key(Target::Godot), written.clone());
        settings
            .per_engine
            .get_mut(key(Target::Godot))
            .unwrap()
            .entities
            .insert(
                "beacon".into(),
                crate::export_bindings::Rule {
                    build: Default::default(),
                    defaults: std::collections::BTreeMap::from([(
                        "label".into(),
                        serde_json::json!("literal \\s and spaces"),
                    )]),
                },
            );
        let written = settings.for_target(Target::Godot);

        let text = settings.to_text();
        std::fs::write(&store, &text).expect("the file is written");
        let back = Settings::load_from(&store).for_target(Target::Godot);

        assert_eq!(back.scale, written.scale, "in {text}");
        assert_eq!(back.collision, written.collision);
        assert_eq!(back.sky, written.sky);
        assert_eq!(back.cull_hidden, written.cull_hidden);
        assert_eq!(back.merge_faces, written.merge_faces);
        assert_eq!(back.static_cells, written.static_cells);
        assert_eq!(back.entities, written.entities);
        assert_eq!(back.merge_collision, written.merge_collision);
        assert_eq!(back.texture_dir, written.texture_dir);
        assert_eq!(back.material_dir, written.material_dir);
        assert_eq!(back.clip_texture, written.clip_texture);
        assert_eq!(back.skip_texture, written.skip_texture);
        assert_eq!(back.origin_texture, written.origin_texture);
        assert_eq!(back.asset_dir, written.asset_dir);
        assert_eq!(back.content_dir, written.content_dir);
        assert_eq!(back.model_material, written.model_material);
        assert_eq!(back.model_sampler, written.model_sampler);
        assert_eq!(
            back.project_dir, written.project_dir,
            "a path with a space in it"
        );
        assert_eq!(back.trigger_layer, written.trigger_layer);
        assert_eq!(back.lightmap_uv2, written.lightmap_uv2);
        assert_eq!(back.lightmap_texel_size, written.lightmap_texel_size);
        assert_eq!(back.name_property, written.name_property);
        assert_eq!(back.group_hierarchy, written.group_hierarchy);
        assert_eq!(back.flip_faces, written.flip_faces);
        assert_eq!(back.flip_uv_v, written.flip_uv_v);
        assert_eq!(
            back.directory, written.directory,
            "a path with a space in it"
        );
        assert_eq!(back.export_on_save, written.export_on_save);

        let _ = std::fs::remove_file(&store);
    }

    #[test]
    fn a_key_from_a_newer_build_is_stepped_over() {
        let store =
            std::env::temp_dir().join(format!("heron-settings-newer-{}", std::process::id()));
        std::fs::write(&store, "godot=scale=64 something_new=yes cull=0\n").expect("written");

        let godot = Settings::load_from(&store).for_target(Target::Godot);
        assert_eq!(godot.scale, 64.0, "the keys around it still read");
        assert!(!godot.cull_hidden);

        let _ = std::fs::remove_file(&store);
    }
}
