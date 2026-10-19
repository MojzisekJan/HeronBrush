//! The maps the user has starred in the welcome window, so they float to the top.
//!
//! Stores one favourite map path per line beside `recent-documents` in the configuration directory.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Where the list lives, beside the preferences file.
pub fn default_path() -> PathBuf {
    crate::preferences::config_dir().join("favourite-maps")
}

/// The starred paths that are still readable files.
pub fn load() -> BTreeSet<PathBuf> {
    load_from(&default_path())
}

fn load_from(store: &Path) -> BTreeSet<PathBuf> {
    let Ok(text) = crate::files::read_to_string(store) else {
        return BTreeSet::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|path| crate::files::is_file(path))
        .collect()
}

/// Whether `path` is starred.
pub fn is_favourite(path: &Path) -> bool {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    load().contains(&canonical)
}

/// Stars `path` if it is not starred, unstars it if it is. Returns the new state.
pub fn toggle(path: &Path) -> bool {
    toggle_in(&default_path(), path)
}

fn toggle_in(store: &Path, path: &Path) -> bool {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut paths = load_from(store);
    let now_on = if paths.contains(&canonical) {
        paths.remove(&canonical);
        false
    } else {
        paths.insert(canonical);
        true
    };
    write(store, &paths);
    now_on
}

/// Drops `path` from the list, e.g. when the map is deleted.
pub fn forget(path: &Path) {
    let store = default_path();
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut paths = load_from(&store);
    if paths.remove(&canonical) || paths.remove(path) {
        write(&store, &paths);
    }
}

fn write(store: &Path, paths: &BTreeSet<PathBuf>) {
    if let Some(directory) = store.parent() {
        let _ = crate::files::create_dir_all(directory);
    }
    let body: String = paths.iter().map(|p| format!("{}\n", p.display())).collect();
    let _ = crate::files::write(store, body);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("heron-fav-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    #[test]
    fn a_star_toggles_on_then_off() {
        let dir = scratch("toggle");
        let store = dir.join("favourite-maps");
        let map = dir.join("m.map");
        std::fs::write(&map, "").unwrap();

        assert!(toggle_in(&store, &map), "first toggle turns it on");
        assert!(load_from(&store).iter().any(|p| p.ends_with("m.map")));
        assert!(!toggle_in(&store, &map), "second toggle turns it off");
        assert!(load_from(&store).is_empty());
    }

    #[test]
    fn a_starred_path_that_is_gone_is_not_returned() {
        let dir = scratch("gone");
        let store = dir.join("favourite-maps");
        let map = dir.join("m.map");
        std::fs::write(&map, "").unwrap();
        toggle_in(&store, &map);
        std::fs::remove_file(&map).unwrap();
        assert!(load_from(&store).is_empty());
    }
}
