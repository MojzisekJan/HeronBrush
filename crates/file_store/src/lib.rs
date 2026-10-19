//! The files the editor reads and writes: maps, the preferences, the recent and favourite
//! lists, and the data a package ships (games, prefabs, materials).
//!
//! On the desktop these are plain `std::fs` calls. The web build has no file system, so
//! there the files live in memory: the page puts them in place before the editor starts
//! ([`preload`]), and every write goes to a hook ([`set_write_hook`]) that stores it in the
//! browser and, for a map the user picked, writes it to that file or downloads it
//! (`heron-brush/src/web.rs`). A file that is not in memory can come from a fallback
//! ([`set_fallback`]): the examples bundled in the binary.

use std::io;
use std::path::{Path, PathBuf};

/// Immutable file snapshot. Keeping the Vec inside Arc preserves its payload allocation.
pub type SharedBytes = std::sync::Arc<Vec<u8>>;

/// Reads an immutable snapshot. Virtual files share their existing buffer; native
/// files are read once into a new snapshot. Replacements do not mutate old snapshots.
pub fn read_shared(path: &Path) -> io::Result<SharedBytes> {
    #[cfg(target_arch = "wasm32")]
    return web::read_shared(path);
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::read(path).map(std::sync::Arc::new)
}

/// Whether a path names a root in this platform's file store.
/// wasm32's standard library has no absolute filesystem paths; our virtual
/// filesystem uses slash-rooted paths for both imports and bundled resources.
pub fn is_absolute(path: &Path) -> bool {
    #[cfg(target_arch = "wasm32")]
    return path.to_string_lossy().starts_with('/');
    #[cfg(not(target_arch = "wasm32"))]
    path.is_absolute()
}

/// Reads a whole file.
pub fn read(path: &Path) -> io::Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    return web::read(path);
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::read(path)
}

/// Reads a whole text file.
pub fn read_to_string(path: &Path) -> io::Result<String> {
    String::from_utf8(read(path)?)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// Writes a whole file, replacing what was there.
pub fn write(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    #[cfg(target_arch = "wasm32")]
    return web::write(path, contents.as_ref());
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::write(path, contents)
}

/// Whether `path` is a file that can be read.
pub fn is_file(path: &Path) -> bool {
    #[cfg(target_arch = "wasm32")]
    return web::is_file(path);
    #[cfg(not(target_arch = "wasm32"))]
    path.is_file()
}

/// Whether `path` is a folder. On the web, one that holds a file.
pub fn is_dir(path: &Path) -> bool {
    #[cfg(target_arch = "wasm32")]
    return web::is_dir(path);
    #[cfg(not(target_arch = "wasm32"))]
    path.is_dir()
}

/// What a folder holds, files and folders, sorted by path.
pub fn read_dir(path: &Path) -> io::Result<Vec<PathBuf>> {
    #[cfg(target_arch = "wasm32")]
    return web::read_dir(path);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(path)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .collect();
        entries.sort();
        Ok(entries)
    }
}

/// Deletes a file.
pub fn remove_file(path: &Path) -> io::Result<()> {
    #[cfg(target_arch = "wasm32")]
    return web::remove(path);
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::remove_file(path)
}

/// Renames a file, replacing whatever was at `to`.
pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let contents = web::read(from)?;
        web::write(to, &contents)?;
        web::remove(from)
    }
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::rename(from, to)
}

/// Makes a folder and its parents. Folders need no making on the web.
pub fn create_dir_all(path: &Path) -> io::Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::create_dir_all(path)
}

#[cfg(target_arch = "wasm32")]
pub use web::{changes_since, preload, set_fallback, set_write_hook, store, visit_changes_since};

#[cfg(any(target_arch = "wasm32", test))]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod web {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::io;
    use std::path::{Path, PathBuf};

    thread_local! {
        static FILES: RefCell<BTreeMap<PathBuf, super::SharedBytes>> = const { RefCell::new(BTreeMap::new()) };
        static CHANGED: RefCell<BTreeMap<PathBuf, u64>> = const { RefCell::new(BTreeMap::new()) };
        static REVISION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
        static HOOK: RefCell<Option<fn(&Path, Option<&[u8]>)>> = const { RefCell::new(None) };
        static FALLBACK: RefCell<Option<fn(&Path) -> Option<&'static [u8]>>> = const { RefCell::new(None) };
    }

    /// Puts files in place before the editor starts: those the page kept from earlier
    /// visits, and the data the package ships.
    pub fn preload(files: impl IntoIterator<Item = (PathBuf, Vec<u8>)>) {
        for (path, contents) in files {
            store(path, contents);
        }
    }

    /// Sets what every [`super::write`] (with the new contents) and every
    /// [`super::remove_file`] (with none) is handed once memory holds the change.
    pub fn set_write_hook(hook: fn(&Path, Option<&[u8]>)) {
        HOOK.with(|slot| *slot.borrow_mut() = Some(hook));
    }

    /// Sets where a file that is not in memory is looked for next.
    pub fn set_fallback(fallback: fn(&Path) -> Option<&'static [u8]>) {
        FALLBACK.with(|slot| *slot.borrow_mut() = Some(fallback));
    }

    /// Puts a file in memory without handing it to the hook: one the page brought in
    /// itself, such as a map the user picked, which it stores on its own.
    pub fn store(path: PathBuf, contents: Vec<u8>) {
        FILES.with(|all| {
            all.borrow_mut()
                .insert(path.clone(), std::sync::Arc::new(contents))
        });
        changed(&path);
    }

    fn changed(path: &Path) {
        let revision = REVISION.with(|version| {
            let next = version
                .get()
                .checked_add(1)
                .expect("virtual file revision overflow");
            version.set(next);
            next
        });
        CHANGED.with(|files| files.borrow_mut().insert(path.to_path_buf(), revision));
    }

    /// A coalesced snapshot of files changed after `since`, including deletions.
    /// Revision zero sends all current files; unchanged large images are not copied.
    pub fn changes_since(since: u64) -> (u64, Vec<(PathBuf, Option<Vec<u8>>)>) {
        let mut changes = Vec::new();
        let revision = visit_changes_since(since, |path, bytes| {
            changes.push((path.to_path_buf(), bytes.map(<[u8]>::to_vec)));
        });
        (revision, changes)
    }

    /// Visit coalesced changes without cloning their contents. The callback must
    /// not mutate the virtual file store while its files are borrowed.
    pub fn visit_changes_since(since: u64, mut visit: impl FnMut(&Path, Option<&[u8]>)) -> u64 {
        let revision = REVISION.with(std::cell::Cell::get);
        CHANGED.with(|changed| {
            FILES.with(|all| {
                let all = all.borrow();
                for (path, _) in changed
                    .borrow()
                    .iter()
                    .filter(|(_, version)| **version > since)
                {
                    visit(path, all.get(path).map(|bytes| bytes.as_slice()));
                }
            })
        });
        revision
    }

    fn fallback(path: &Path) -> Option<&'static [u8]> {
        FALLBACK
            .with(|slot| *slot.borrow())
            .and_then(|fallback| fallback(path))
    }

    pub fn read_shared(path: &Path) -> io::Result<super::SharedBytes> {
        if let Some(contents) = FILES.with(|all| all.borrow().get(path).cloned()) {
            return Ok(contents);
        }
        fallback(path)
            .map(|bytes| std::sync::Arc::new(bytes.to_vec()))
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    pub fn read(path: &Path) -> io::Result<Vec<u8>> {
        read_shared(path).map(|bytes| bytes.as_ref().clone())
    }

    pub fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
        store(path.to_path_buf(), contents.to_vec());
        if let Some(hook) = HOOK.with(|slot| *slot.borrow()) {
            hook(path, Some(contents));
        }
        Ok(())
    }

    pub fn remove(path: &Path) -> io::Result<()> {
        if FILES.with(|all| all.borrow_mut().remove(path)).is_none() {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        changed(path);
        if let Some(hook) = HOOK.with(|slot| *slot.borrow()) {
            hook(path, None);
        }
        Ok(())
    }

    pub fn is_file(path: &Path) -> bool {
        FILES.with(|all| all.borrow().contains_key(path)) || fallback(path).is_some()
    }

    pub fn is_dir(path: &Path) -> bool {
        FILES.with(|all| {
            all.borrow()
                .range(path.to_path_buf()..)
                .next()
                .is_some_and(|(file, _)| file != path && file.starts_with(path))
        })
    }

    /// The files and folders right inside `path`: every file below it, cut down to the
    /// part one level in.
    pub fn read_dir(path: &Path) -> io::Result<Vec<PathBuf>> {
        let entries: std::collections::BTreeSet<PathBuf> = FILES.with(|all| {
            all.borrow()
                .range(path.to_path_buf()..)
                .take_while(|(file, _)| file.starts_with(path))
                .filter_map(|(file, _)| {
                    file.strip_prefix(path)
                        .ok()?
                        .components()
                        .next()
                        .map(|first| path.join(first))
                })
                .collect()
        });
        if entries.is_empty() && !is_dir(path) {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        Ok(entries.into_iter().collect())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn shared_snapshots_preserve_payload_and_survive_replacement_and_removal() {
            let path = Path::new("/shared-archive.pak");
            let payload = vec![7; 1024 * 1024];
            let pointer = payload.as_ptr();
            store(path.to_path_buf(), payload);
            let first = read_shared(path).unwrap();
            let second = read_shared(path).unwrap();
            assert_eq!(first.as_ptr(), pointer);
            assert!(std::sync::Arc::ptr_eq(&first, &second));
            let mut owned = read(path).unwrap();
            assert_ne!(owned.as_ptr(), pointer);
            owned[0] = 9;
            assert_eq!(first[0], 7);
            store(path.to_path_buf(), vec![3; 32]);
            let replacement = read_shared(path).unwrap();
            assert!(!std::sync::Arc::ptr_eq(&first, &replacement));
            remove(path).unwrap();
            assert!(read_shared(path).is_err());
            assert_eq!(replacement.as_slice(), &[3; 32]);
            assert_eq!(first.as_slice(), vec![7; 1024 * 1024]);
        }

        #[test]
        fn snapshots_coalesce_edits_and_deletions_without_copying_unchanged_images() {
            FILES.with(|all| all.borrow_mut().clear());
            CHANGED.with(|all| all.borrow_mut().clear());
            REVISION.with(|r| r.set(0));
            preload([
                (PathBuf::from("/image.png"), vec![1; 1024]),
                (PathBuf::from("/map.map"), vec![2]),
            ]);
            let (initial, files) = changes_since(0);
            assert_eq!(files.len(), 2);
            assert!(changes_since(initial).1.is_empty());
            let mut visited = 0;
            assert_eq!(
                visit_changes_since(0, |path, bytes| {
                    FILES.with(|all| {
                        assert_eq!(bytes.unwrap().as_ptr(), all.borrow()[path].as_ptr())
                    });
                    visited += 1;
                }),
                initial
            );
            assert_eq!(visited, 2);
            write(Path::new("/map.map"), &[3]).unwrap();
            write(Path::new("/map.map"), &[4]).unwrap();
            let (edited, files) = changes_since(initial);
            assert_eq!(files, vec![(PathBuf::from("/map.map"), Some(vec![4]))]);
            remove(Path::new("/map.map")).unwrap();
            assert_eq!(
                changes_since(edited).1,
                vec![(PathBuf::from("/map.map"), None)]
            );
            let mut deletions = 0;
            visit_changes_since(edited, |path, bytes| {
                assert_eq!(path, Path::new("/map.map"));
                assert!(bytes.is_none());
                deletions += 1;
            });
            assert_eq!(deletions, 1);
            assert_eq!(read(Path::new("/image.png")).unwrap().len(), 1024);
        }
    }
}
