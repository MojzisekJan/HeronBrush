//! Persisted activation by stable installation ID, independent of display names.
use std::{collections::BTreeSet, path::Path};

pub fn load(path: &Path) -> Result<BTreeSet<String>, String> {
    match crate::files::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Cannot read plugin activation: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeSet::new()),
        Err(e) => Err(format!("Cannot read plugin activation: {e}")),
    }
}
pub fn set_enabled(path: &Path, id: &str, enabled: bool) -> Result<(), String> {
    let mut disabled = load(path)?;
    if enabled {
        disabled.remove(id);
    } else {
        disabled.insert(id.to_owned());
    }
    let parent = path.parent().ok_or("Missing plugin activation directory")?;
    let json = serde_json::to_vec_pretty(&disabled).map_err(|e| e.to_string())?;
    // The web build's files are in memory, written whole at once; on a disk the new
    // store replaces the old in one rename, so a crash cannot leave half of it.
    #[cfg(target_arch = "wasm32")]
    {
        let _ = parent;
        crate::files::write(path, json).map_err(|e| e.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let pending = path.with_extension("pending");
        std::fs::write(&pending, json).map_err(|e| e.to_string())?;
        std::fs::rename(&pending, path).map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activation_roundtrips_and_preserves_other_plugins() {
        let dir = std::env::temp_dir().join(format!("plugin-activation-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        set_enabled(&path, "builtin:checker", false).unwrap();
        set_enabled(&path, "user:checker.rhai", false).unwrap();
        set_enabled(&path, "builtin:checker", true).unwrap();
        let disabled = load(&path).unwrap();
        assert!(!disabled.contains("builtin:checker"));
        assert!(disabled.contains("user:checker.rhai"));
        std::fs::write(&path, "broken").unwrap();
        assert!(set_enabled(&path, "user:checker.rhai", true).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken");
        std::fs::remove_dir_all(dir).unwrap();
    }
}

/// Settings keyed by stable plugin installation ID, independent of display names.
pub type Settings = std::collections::BTreeMap<
    String,
    std::collections::BTreeMap<String, crate::plugin_manifest::SettingValue>,
>;

/// Load persisted answers. A malformed store is reported and never overwritten silently.
pub fn load_settings(path: &Path) -> Result<Settings, String> {
    match crate::files::read(path) {
        Ok(bytes) if bytes.len() <= 1024 * 1024 => {
            serde_json::from_slice(&bytes).map_err(|e| format!("Cannot read plugin settings: {e}"))
        }
        Ok(_) => Err("Plugin settings store exceeds 1 MiB".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::new()),
        Err(e) => Err(format!("Cannot read plugin settings: {e}")),
    }
}
/// Save one plugin's answers while retaining other installations' settings.
pub fn save_settings(
    path: &Path,
    id: &str,
    answers: &[(String, crate::plugin_manifest::SettingValue)],
) -> Result<(), String> {
    let mut settings = load_settings(path)?;
    settings.insert(id.to_owned(), answers.iter().cloned().collect());
    let bytes = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("Plugin settings store exceeds 1 MiB".into());
    }
    #[cfg(target_arch = "wasm32")]
    {
        crate::files::write(path, bytes).map_err(|e| e.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::create_dir_all(path.parent().ok_or("Missing plugin settings directory")?)
            .map_err(|e| e.to_string())?;
        crate::godot_live::atomic_write(path, &bytes).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod settings_tests {
    use super::*;
    use crate::plugin_manifest::SettingValue;
    #[test]
    fn answers_survive_restart_and_corrupt_files_are_retained() {
        let dir =
            std::env::temp_dir().join(format!("heron-plugin-settings-{}", std::process::id()));
        let path = dir.join("answers.json");
        save_settings(
            &path,
            "builtin:bulk-properties",
            &[("value".into(), SettingValue::Text("blue".into()))],
        )
        .unwrap();
        save_settings(
            &path,
            "user:custom",
            &[("count".into(), SettingValue::Number(9.0))],
        )
        .unwrap();
        let restarted = load_settings(&path).unwrap();
        assert_eq!(
            restarted["builtin:bulk-properties"]["value"],
            SettingValue::Text("blue".into())
        );
        assert_eq!(restarted["user:custom"]["count"], SettingValue::Number(9.0));
        std::fs::write(&path, "broken").unwrap();
        assert!(save_settings(&path, "user:custom", &[]).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
