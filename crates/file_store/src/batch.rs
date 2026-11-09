//! Stage a complete native export before replacing any of its files.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

struct Staged {
    target: PathBuf,
    directory: tempfile::TempDir,
    existed: bool,
}

/// Serializes cooperating exports to a directory and rolls back a failed commit.
pub fn write_batch_atomic(directory: &Path, files: &[(PathBuf, &[u8])]) -> io::Result<()> {
    write_batch_with(directory, files, |from, to| fs::rename(from, to))
}

fn write_batch_with(
    directory: &Path,
    files: &[(PathBuf, &[u8])],
    mut replace: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    // Keep the lock inode in place. Removing it would let another export lock a
    // different inode while an existing waiter still holds this one.
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join(".heron-export.lock"))?;
    lock.try_lock().map_err(io::Error::other)?;
    let mut staged = Vec::new();
    let mut targets = std::collections::HashSet::new();
    for (path, bytes) in files {
        let target = match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => path.canonicalize()?,
            _ => path.clone(),
        };
        let parent = target
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let target = parent
            .canonicalize()?
            .join(target.file_name().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Export target has no file name",
                )
            })?);
        if !targets.insert(target.clone()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Duplicate export target",
            ));
        }
        let metadata = match fs::metadata(&target) {
            Ok(metadata) => {
                if !metadata.is_file() || metadata.permissions().readonly() {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("Cannot replace {}", target.display()),
                    ));
                }
                Some(metadata)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        let directory = tempfile::Builder::new()
            .prefix(".heron-export-")
            .tempdir_in(parent)?;
        let mut file = File::create(directory.path().join("new"))?;
        file.write_all(bytes)?;
        if let Some(metadata) = &metadata {
            file.set_permissions(metadata.permissions())?;
            let backup = directory.path().join("old");
            // A hard link retains the previous inode without duplicating its payload.
            if fs::hard_link(&target, &backup).is_err() {
                fs::copy(&target, &backup)?;
                File::open(&backup)?.sync_all()?;
            }
        }
        file.sync_all()?;
        staged.push(Staged {
            target,
            directory,
            existed: metadata.is_some(),
        });
    }
    for (index, entry) in staged.iter().enumerate() {
        if let Err(error) = replace(&entry.directory.path().join("new"), &entry.target) {
            let failed_target = entry.target.clone();
            let mut recovery = Vec::new();
            for old in staged.drain(..index).rev() {
                let restored = if old.existed {
                    fs::rename(old.directory.path().join("old"), &old.target)
                } else {
                    fs::remove_file(&old.target)
                };
                if let Err(reason) = restored {
                    // Retain the original bytes if another filesystem failure prevents rollback.
                    let saved = old.directory.keep();
                    recovery.push(format!(
                        "{}: {reason}; backup in {}",
                        old.target.display(),
                        saved.display()
                    ));
                }
            }
            return Err(io::Error::new(
                error.kind(),
                format!(
                    "Could not replace {}: {error}{}",
                    failed_target.display(),
                    if recovery.is_empty() {
                        String::new()
                    } else {
                        format!("; {}", recovery.join("; "))
                    }
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outputs(root: &Path) -> Vec<(PathBuf, &'static [u8])> {
        vec![
            (root.join("scene"), b"new scene"),
            (root.join("mesh"), b"new mesh"),
        ]
    }

    #[test]
    fn batch_staging_failure_preserves_every_existing_file() {
        let root = tempfile::tempdir().unwrap();
        let files = outputs(root.path());
        fs::write(&files[0].0, b"original scene").unwrap();
        fs::create_dir(&files[1].0).unwrap();
        assert!(write_batch_atomic(root.path(), &files).is_err());
        assert_eq!(fs::read(&files[0].0).unwrap(), b"original scene");
        assert!(files[1].0.is_dir());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 3);
    }

    #[test]
    fn batch_commit_failure_restores_old_files_removes_new_files_and_can_retry() {
        let root = tempfile::tempdir().unwrap();
        let files = vec![
            (root.path().join("scene"), b"new scene".as_slice()),
            (root.path().join("new texture"), b"texture".as_slice()),
            (root.path().join("mesh"), b"new mesh".as_slice()),
        ];
        fs::write(&files[0].0, b"original scene").unwrap();
        fs::write(&files[2].0, b"original mesh").unwrap();
        let error = write_batch_with(root.path(), &files, |from, to| {
            if to.file_name() == files[2].0.file_name() {
                Err(io::Error::other("simulated commit failure"))
            } else {
                fs::rename(from, to)
            }
        })
        .unwrap_err();
        assert!(error.to_string().contains("simulated commit failure"));
        assert_eq!(fs::read(&files[0].0).unwrap(), b"original scene");
        assert_eq!(fs::read(&files[2].0).unwrap(), b"original mesh");
        assert!(!files[1].0.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 3);
        write_batch_atomic(root.path(), &files).unwrap();
        for (path, bytes) in files {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }

    #[test]
    fn batch_rejects_a_concurrent_export_and_preserves_permissions_and_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let files = outputs(root.path());
        let lock = File::create(root.path().join(".heron-export.lock")).unwrap();
        lock.lock().unwrap();
        assert!(write_batch_atomic(root.path(), &files).is_err());
        assert!(!files[0].0.exists());
        drop(lock);
        write_batch_atomic(root.path(), &files).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::{symlink, PermissionsExt};
            fs::set_permissions(&files[0].0, fs::Permissions::from_mode(0o640)).unwrap();
            let link = root.path().join("link");
            symlink(&files[0].0, &link).unwrap();
            write_batch_atomic(root.path(), &[(link.clone(), b"replacement")]).unwrap();
            assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
            assert_eq!(fs::read(&files[0].0).unwrap(), b"replacement");
            assert_eq!(
                fs::metadata(&files[0].0).unwrap().permissions().mode() & 0o777,
                0o640
            );
        }
    }
}
