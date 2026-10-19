//! The plugins the editor knows about: the bundled starter set and a person's own
//! scripts under `~/.config/heron-brush/plugins/`, with which of them are switched off.
//!
//! A folder plugin may also carry a `plugin.toml` naming itself and the editor API it
//! was written against ([`crate::plugin_manifest`]); one written for an API this build
//! does not offer is listed and refused rather than run. Installing copies a `.rhai` file
//! or a whole plugin folder into the plugins directory.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use crate::i18n::tr;
use crate::plugin_manifest::{Manifest, Setting};

/// One plugin the window offers.
#[derive(Clone)]
pub struct Plugin {
    /// What it is called: the manifest's name, or a bundled script's stem, or a user
    /// file's stem.
    pub id: String,
    pub enabled: bool,
    pub path: Option<PathBuf>,
    pub name: String,
    /// A one-line summary: the manifest's description, or the script's first `//` line.
    pub summary: String,
    /// The plugin's own version, when its manifest gives one.
    pub version: Option<String>,
    /// The API version it asks for, when that is not the one this build offers. `Some`
    /// here means the plugin is listed but cannot be run.
    pub wrong_api: Option<u32>,
    /// What it wants asked before it runs, in the order the manifest declared them.
    pub settings: Vec<Setting>,
    pub actions: Vec<crate::plugin_manifest::Action>,
    /// Optional readout action run on an isolated snapshot by the visible inspector.
    pub panel_action: Option<String>,
    /// The script itself.
    pub source: String,
}

/// The scripts compiled into the binary, the starter set: a name to fall back on, the
/// script, and the manifest text, empty for a plugin that declares nothing.
pub const BUNDLED: &[(&str, &str, &str)] = &[
    ("bulk-properties", include_str!("../../../assets/plugins/bulk-properties/main.rhai"), include_str!("../../../assets/plugins/bulk-properties/plugin.toml")),
    ("project-rules", include_str!("../../../assets/plugins/project-rules/main.rhai"), include_str!("../../../assets/plugins/project-rules/plugin.toml")),
    (
        "selection-inspector",
        include_str!("../../../assets/plugins/selection-inspector/main.rhai"),
        include_str!("../../../assets/plugins/selection-inspector/plugin.toml"),
    ),
    (
        "desaturate",
        include_str!("../../../assets/plugins/desaturate.rhai"),
        "",
    ),
    (
        "checker",
        include_str!("../../../assets/plugins/checker.rhai"),
        "",
    ),
    (
        "recolour by height",
        include_str!("../../../assets/plugins/recolour_by_height.rhai"),
        "",
    ),
    (
        "stack",
        include_str!("../../../assets/plugins/stack/main.rhai"),
        include_str!("../../../assets/plugins/stack/plugin.toml"),
    ),
    (
        "export face list",
        include_str!("../../../assets/plugins/export-face-list/main.rhai"),
        include_str!("../../../assets/plugins/export-face-list/plugin.toml"),
    ),
];

/// Where a person's own plugins live.
pub fn user_directory() -> PathBuf {
    crate::preferences::config_dir().join("plugins")
}

/// The script's first `//` comment paragraph, trimmed and joined into one line, as its
/// summary: a sentence the author wrapped over two comment lines is kept whole rather
/// than cut off mid-sentence. The paragraph ends at an empty `//` line or the first line
/// that is not a comment. Empty when the script has no leading comment.
pub fn summarise(source: &str) -> String {
    let lines = source.lines().map(str::trim).skip_while(|line| !line.starts_with("//"));
    let mut parts: Vec<&str> = Vec::new();
    for line in lines {
        let Some(text) = line.strip_prefix("//") else { break };
        let text = text.trim_start_matches('/').trim();
        if text.is_empty() {
            break;
        }
        parts.push(text);
    }
    parts.join(" ")
}

/// Every plugin found: the bundled set, then a person's own from the config directory,
/// sorted by name within each group.
pub fn discover() -> Vec<Plugin> {
    let disabled = crate::plugin_state::load(&activation_path());
    let mut plugins: Vec<Plugin> = BUNDLED
        .iter()
        .map(|(name, source, manifest)| {
            let manifest = Manifest::parse(manifest);
            let id = format!("builtin:{name}");
            Plugin {
                enabled: disabled.as_ref().is_ok_and(|set| !set.contains(&id)),
                id,
                path: None,
                name: manifest.name.clone().unwrap_or_else(|| (*name).to_string()),
                summary: manifest
                    .description
                    .clone()
                    .unwrap_or_else(|| summarise(source)),
                version: manifest.version.clone(),
                wrong_api: (!manifest.api_matches()).then_some(manifest.api).flatten(),
                settings: manifest.settings,
                actions: manifest.actions,
                panel_action: manifest.panel_action,
                source: (*source).to_string(),
            }
        })
        .collect();

    let mut mine: Vec<Plugin> = Vec::new();
    if let Ok(entries) = crate::files::read_dir(&user_directory()) {
        for path in entries {
            let script = if path.extension().is_some_and(|e| e == "rhai") {
                path.clone()
            } else if crate::files::is_dir(&path) && crate::files::is_file(&path.join("main.rhai")) {
                path.join("main.rhai")
            } else {
                continue;
            };
            let Ok(source) = crate::files::read_to_string(&script) else {
                continue;
            };
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "plugin".to_string());
            // A `.rhai` file on its own has no folder to hold a manifest, so only a
            // folder plugin can carry one.
            let manifest = Manifest::beside(&script).unwrap_or_default();
            let id = format!(
                "user:{}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            mine.push(Plugin {
                enabled: disabled.as_ref().is_ok_and(|set| !set.contains(&id)),
                id,
                path: Some(script.clone()),
                name: manifest.name.clone().unwrap_or(stem),
                summary: manifest
                    .description
                    .clone()
                    .unwrap_or_else(|| summarise(&source)),
                version: manifest.version.clone(),
                wrong_api: (!manifest.api_matches()).then_some(manifest.api).flatten(),
                settings: manifest.settings,
                actions: manifest.actions,
                panel_action: manifest.panel_action,
                source,
            });
        }
    }
    mine.sort_by(|a, b| a.name.cmp(&b.name));
    plugins.extend(mine);
    plugins
}

/// Copies what a person chose into the plugins directory: a `.rhai` file as it is, a
/// folder with everything in it. Returns the name it landed under.
pub fn install(chosen: &Path) -> Result<String, String> {
    let destination = user_directory();
    crate::files::create_dir_all(&destination).map_err(|error| error.to_string())?;
    let name = chosen
        .file_name()
        .ok_or_else(|| "that has no name".to_string())?;
    let landing = destination.join(name);
    if crate::files::is_dir(chosen) {
        if !crate::files::is_file(&chosen.join("main.rhai")) {
            return Err(tr("a plugin folder needs a main.rhai in it").to_string());
        }
        copy_tree(chosen, &landing).map_err(|error| error.to_string())?;
    } else {
        if chosen.extension().is_none_or(|e| e != "rhai") {
            return Err(tr("a plugin is a .rhai file or a folder holding one").to_string());
        }
        crate::files::write(&landing, crate::files::read(chosen).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    }
    Ok(name.to_string_lossy().into_owned())
}

/// Copies a directory and everything under it, the way a plugin folder wants copying.
pub fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    crate::files::create_dir_all(to)?;
    for source in crate::files::read_dir(from)? {
        let Some(name) = source.file_name() else { continue };
        let target = to.join(name);
        if crate::files::is_dir(&source) {
            copy_tree(&source, &target)?;
        } else {
            crate::files::write(&target, crate::files::read(&source)?)?;
        }
    }
    Ok(())
}

thread_local! { static ACTIVATION_PATH: RefCell<Option<PathBuf>> = const { RefCell::new(None) }; }

/// Points the activation store somewhere else, for tests that must not touch the real one.
pub fn override_activation_path(path: Option<PathBuf>) {
    ACTIVATION_PATH.with(|p| *p.borrow_mut() = path);
}

/// Where which plugins are switched off is kept.
pub fn activation_path() -> PathBuf {
    if let Some(path) = ACTIVATION_PATH.with(|p| p.borrow().clone()) {
        return path;
    }
    crate::preferences::config_dir().join("plugin-activation.json")
}
pub fn enabled(id: &str) -> bool {
    crate::plugin_state::load(&activation_path()).is_ok_and(|disabled| !disabled.contains(id))
}
pub fn check_path_enabled(path: &Path) -> Result<(), String> {
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    if discover().iter().any(|p| {
        !p.enabled
            && p.path.as_ref().and_then(|p| p.canonicalize().ok()).as_ref() == Some(&canonical)
    }) {
        return Err("Plugin is disabled".into());
    }
    Ok(())
}


#[cfg(test)]
mod summary_tests {
    use super::summarise;

    #[test]
    fn a_wrapped_first_comment_is_kept_whole() {
        assert_eq!(summarise("// Do a thing.\nlet x = 1;"), "Do a thing.");
        assert_eq!(summarise("let x = 1; // trailing"), "");
        assert_eq!(summarise("// Paint faces, for blocking out a\n// level first.\n//\n// Run it."), "Paint faces, for blocking out a level first.");
    }
}
