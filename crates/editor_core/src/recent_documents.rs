//! The list of recently opened maps, shown in the welcome window.
//!
//! Stores readable recent map paths, newest first, in a bounded list with one path per line in the
//! configuration directory.

use std::path::{Path, PathBuf};

/// How many paths are kept. `createRecentDocuments` passes 10.
const MAX: usize = 10;

/// Where the list lives, beside the preferences file.
pub fn default_path() -> PathBuf {
    crate::preferences::config_dir().join("recent-documents")
}

/// The recent documents, newest first, dropping any that is no longer a readable file.
///
/// `RecentDocuments::updateFilteredDocuments` with `checkRecentDocument`: a path that has
/// since been moved or deleted is filtered out rather than shown and then failing on a
/// click.
pub fn load() -> Vec<PathBuf> {
    load_from(&default_path())
}

fn load_from(store: &Path) -> Vec<PathBuf> {
    let Ok(text) = crate::files::read_to_string(store) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .take(MAX)
        .filter(|path| crate::files::is_file(path))
        .collect()
}

/// Drops `path` from the list, e.g. after the map is deleted or the user dismisses it
/// from the welcome window.
pub fn forget(path: &Path) {
    forget_in(&default_path(), path);
}

fn forget_in(store: &Path, path: &Path) {
    let canonical = normalized(path);
    let Ok(text) = crate::files::read_to_string(store) else {
        return;
    };
    // The raw lines, not `load_from`, so a line for a file that is now gone (the delete
    // case) is what this removes.
    let kept: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| {
            let p = Path::new(line);
            p != canonical && p != path
        })
        .collect();
    let body: String = kept.iter().map(|line| format!("{line}\n")).collect();
    let _ = crate::files::write(store, body);
}

/// Moves `path` to the front of the list and writes it back.
///
/// `RecentDocuments::updatePath` then `insertPath`: the path is normalised, any earlier
/// copy of it removed, it goes on the front, and the list is capped.
pub fn record(path: &Path) {
    record_in(&default_path(), path);
}

/// The path as the list writes it: canonical when it can be.
///
/// A path whose file is already gone -- the delete-then-forget case -- cannot be
/// canonicalised whole, but its parent still can, and on macOS it must be: the
/// temporary directory is reached through the `/var` -> `/private/var` symlink, so the
/// recorded (canonical) line and the naive fallback would name the same file two ways
/// and the forget would miss.
fn normalized(path: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }
    if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
        if let Ok(parent) = std::fs::canonicalize(parent) {
            return parent.join(name);
        }
    }
    path.to_path_buf()
}

fn record_in(store: &Path, path: &Path) {
    let canonical = normalized(path);
    let mut paths = load_from(store);
    paths.retain(|existing| existing != &canonical);
    paths.insert(0, canonical);
    paths.truncate(MAX);

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
        let dir = std::env::temp_dir().join(format!("heron-recent-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    #[test]
    fn a_recorded_path_comes_back_at_the_front() {
        let dir = scratch("front");
        let store = dir.join("recent-documents");
        let first = dir.join("first.map");
        let second = dir.join("second.map");
        std::fs::write(&first, "").unwrap();
        std::fs::write(&second, "").unwrap();

        record_in(&store, &first);
        record_in(&store, &second);

        let loaded = load_from(&store);
        assert_eq!(loaded.len(), 2);
        assert!(loaded[0].ends_with("second.map"));
        assert!(loaded[1].ends_with("first.map"));
    }

    #[test]
    fn recording_the_same_path_twice_keeps_one_entry() {
        let dir = scratch("dedupe");
        let store = dir.join("recent-documents");
        let map = dir.join("map.map");
        std::fs::write(&map, "").unwrap();

        record_in(&store, &map);
        record_in(&store, &map);

        assert_eq!(load_from(&store).len(), 1);
    }

    #[test]
    fn a_path_that_no_longer_exists_is_filtered_out() {
        let dir = scratch("missing");
        let store = dir.join("recent-documents");
        let map = dir.join("gone.map");
        std::fs::write(&map, "").unwrap();
        record_in(&store, &map);
        std::fs::remove_file(&map).unwrap();

        assert!(load_from(&store).is_empty());
    }

    #[test]
    fn a_forgotten_path_is_dropped_even_after_the_file_is_gone() {
        let dir = scratch("forget");
        let store = dir.join("recent-documents");
        let a = dir.join("a.map");
        let b = dir.join("b.map");
        std::fs::write(&a, "").unwrap();
        std::fs::write(&b, "").unwrap();
        record_in(&store, &a);
        record_in(&store, &b);

        std::fs::remove_file(&b).unwrap();
        forget_in(&store, &b);

        std::fs::write(&b, "").unwrap(); // recreate; its line should still be gone
        let loaded = load_from(&store);
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].ends_with("a.map"));
    }

    #[test]
    fn the_list_is_capped() {
        let dir = scratch("cap");
        let store = dir.join("recent-documents");
        for i in 0..(MAX + 5) {
            let map = dir.join(format!("map{i}.map"));
            std::fs::write(&map, "").unwrap();
            record_in(&store, &map);
        }
        assert_eq!(load_from(&store).len(), MAX);
    }
}
