//! Indexed game assets shared by model, palette and material readers.
//!
//! Roots are ordered from base game to highest-priority mod. Within a root,
//! loose files override archives, and naturally later archive names override earlier ones.
//! Archive members are never extracted onto the host filesystem.

use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

#[path = "asset_zip.rs"]
mod zip;

/// A file with its winning source already resolved.
#[derive(Clone)]
pub struct AssetFile {
    /// Normalized game-relative path, retaining the source's spelling.
    pub name: String,
    /// Loose file or archive containing this asset.
    pub origin: PathBuf,
    source: Source,
}

#[derive(Clone)]
enum Source {
    Loose,
    Archive(Arc<Archive>, usize),
}
enum Archive {
    Pak(crate::pak::Pak),
    #[cfg(not(target_arch = "wasm32"))]
    PakFile(crate::pak::FilePak),
    Zip(zip::Zip),
}

impl std::fmt::Debug for AssetFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetFile")
            .field("name", &self.name)
            .field("origin", &self.origin)
            .finish()
    }
}
impl PartialEq for AssetFile {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.origin == other.origin
            && match (&self.source, &other.source) {
                (Source::Loose, Source::Loose) => true,
                (Source::Archive(a, i), Source::Archive(b, j)) => i == j && Arc::ptr_eq(a, b),
                _ => false,
            }
    }
}
impl Eq for AssetFile {}

impl AssetFile {
    /// Reads this file, inflating only this member if necessary.
    pub fn read(&self) -> Result<Vec<u8>, String> {
        match &self.source {
            Source::Loose => file_store::read(&self.origin).map_err(|e| e.to_string()),
            Source::Archive(archive, index) => match archive.as_ref() {
                Archive::Pak(pak) => {
                    let bytes = pak.read_index(*index).ok_or("missing PAK entry")?;
                    if bytes.len() > crate::pak::MAX_MEMBER_BYTES {
                        return Err("PAK member exceeds 256 MiB".into());
                    }
                    Ok(bytes.to_vec())
                }
                #[cfg(not(target_arch = "wasm32"))]
                Archive::PakFile(pak) => pak.read_index(*index),
                Archive::Zip(zip) => zip.read(*index),
            },
        }
    }
    /// Provenance for diagnostics and texture cache keys. Not an extractable disk path.
    pub fn display_path(&self) -> PathBuf {
        match self.source {
            Source::Loose => self.origin.clone(),
            Source::Archive(..) => {
                PathBuf::from(format!("{}!/{}", self.origin.display(), self.name))
            }
        }
    }
    /// Whether the asset is stored in an archive.
    pub fn is_archived(&self) -> bool {
        matches!(self.source, Source::Archive(..))
    }
}

/// Winning resources, indexed once with case-insensitive game-relative names.
#[derive(Default, Debug)]
pub struct AssetFileSystem {
    files: BTreeMap<String, AssetFile>,
    warnings: Vec<String>,
}

impl AssetFileSystem {
    /// Mounts roots in increasing priority. Invalid archives are reported and skipped.
    pub fn mount(roots: &[PathBuf]) -> Self {
        let mut result = Self::default();
        let mut seen_roots = HashSet::new();
        let mut unique = Vec::new();
        for root in roots.iter().rev() {
            let identity = root.canonicalize().unwrap_or_else(|_| root.clone());
            if seen_roots.insert(identity) {
                unique.push(root);
            }
        }
        for root in unique.into_iter().rev() {
            if root.as_os_str().is_empty() {
                continue;
            }
            let entries = match file_store::read_dir(root) {
                Ok(entries) => entries,
                Err(error) => {
                    result.warnings.push(format!("{}: {error}", root.display()));
                    continue;
                }
            };
            let mut packages: Vec<_> = entries
                .into_iter()
                .filter(|p| {
                    p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                        ["pak", "pk3", "zip"]
                            .iter()
                            .any(|known| e.eq_ignore_ascii_case(known))
                    })
                })
                .collect();
            packages.sort_by(|a, b| {
                natural_cmp(
                    &a.file_name().unwrap_or_default().to_string_lossy(),
                    &b.file_name().unwrap_or_default().to_string_lossy(),
                )
                .then(a.cmp(b))
            });
            for path in packages {
                result.mount_archive(&path);
            }
            let mut files = Vec::new();
            walk(root, &mut HashSet::new(), &mut files, &mut result.warnings);
            files.sort();
            for path in files {
                let Ok(relative) = path.strip_prefix(root) else {
                    continue;
                };
                let Some(name) = normalize(&relative.to_string_lossy()) else {
                    continue;
                };
                result.files.insert(
                    name.to_lowercase(),
                    AssetFile {
                        name,
                        origin: path,
                        source: Source::Loose,
                    },
                );
            }
        }
        result
    }

    fn mount_archive(&mut self, path: &Path) {
        let parse_memory = || {
            file_store::read_shared(path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    if bytes.starts_with(b"PACK") {
                        crate::pak::Pak::read_shared(bytes)
                            .map(Archive::Pak)
                            .map_err(|e| e.to_string())
                    } else {
                        zip::Zip::parse_shared(bytes).map(Archive::Zip)
                    }
                })
        };
        #[cfg(target_arch = "wasm32")]
        let parsed = parse_memory();
        #[cfg(all(test, not(target_arch = "wasm32")))]
        let memory_benchmark = std::env::var("HERON_BENCH_PAK_MEMORY").as_deref() == Ok("1")
            || std::env::var("HERON_BENCH_ZIP_MEMORY").as_deref() == Ok("1");
        #[cfg(all(not(test), not(target_arch = "wasm32")))]
        let memory_benchmark = false;
        #[cfg(not(target_arch = "wasm32"))]
        let parsed = if memory_benchmark {
            parse_memory()
        } else {
            (|| {
                use std::io::Read;
                let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
                let mut magic = [0; 4];
                file.read_exact(&mut magic).map_err(|e| e.to_string())?;
                if &magic == b"PACK" {
                    crate::pak::FilePak::open(path).map(Archive::PakFile)
                } else {
                    zip::Zip::open(path).map(Archive::Zip)
                }
            })()
        };
        let archive = match parsed {
            Ok(archive) => Arc::new(archive),
            Err(error) => {
                self.warnings.push(format!("{}: {error}", path.display()));
                return;
            }
        };
        let names: Vec<String> = match archive.as_ref() {
            Archive::Pak(pak) => pak.entries().iter().map(|e| e.name.clone()).collect(),
            #[cfg(not(target_arch = "wasm32"))]
            Archive::PakFile(pak) => pak.entries().iter().map(|e| e.name.clone()).collect(),
            Archive::Zip(zip) => zip.names().map(str::to_owned).collect(),
        };
        for (index, raw) in names.into_iter().enumerate() {
            let Some(name) = normalize(&raw) else {
                self.warnings
                    .push(format!("{}: invalid asset path {raw:?}", path.display()));
                continue;
            };
            if raw.ends_with('/') {
                continue;
            }
            self.files.insert(
                name.to_lowercase(),
                AssetFile {
                    name,
                    origin: path.to_path_buf(),
                    source: Source::Archive(archive.clone(), index),
                },
            );
        }
    }

    /// Resolves a normalized relative reference; `..` may not escape a root.
    pub fn resolve(&self, name: &str) -> Option<&AssetFile> {
        self.files.get(&normalize(name)?.to_lowercase())
    }
    /// Reads the resource that wins across all roots and archives.
    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.resolve(name)?.read().ok()
    }
    /// Effective files in stable, case-insensitive path order, without decompressing them.
    pub fn files(&self) -> impl Iterator<Item = &AssetFile> {
        self.files.values()
    }
    /// Errors encountered while mounting, including unsupported or damaged archives.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

fn walk(
    root: &Path,
    seen: &mut HashSet<PathBuf>,
    files: &mut Vec<PathBuf>,
    warnings: &mut Vec<String>,
) {
    let identity = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if !seen.insert(identity.clone()) {
        return;
    }
    match file_store::read_dir(root) {
        Ok(entries) => {
            for path in entries {
                if file_store::is_dir(&path) {
                    walk(&path, seen, files, warnings);
                } else if file_store::is_file(&path) {
                    files.push(path);
                }
            }
        }
        Err(error) => warnings.push(format!("{}: {error}", root.display())),
    }
    seen.remove(&identity);
}

/// Normalizes resource references without granting access outside the mounted roots.
pub fn normalize(name: &str) -> Option<String> {
    let name = name.replace('\\', "/");
    if name.starts_with('/') || name.contains(['\0', ':']) {
        return None;
    }
    let mut components = Vec::new();
    for part in name.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                components.pop()?;
            }
            _ => components.push(part),
        }
    }
    (!components.is_empty()).then(|| components.join("/"))
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let (start_i, start_j) = (i, j);
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let (mut x, mut y) = (&a[start_i..i], &b[start_j..j]);
            while x.first() == Some(&b'0') {
                x = &x[1..];
            }
            while y.first() == Some(&b'0') {
                y = &y[1..];
            }
            let order = x.len().cmp(&y.len()).then(x.cmp(y));
            if !order.is_eq() {
                return order;
            }
        } else {
            let order = a[i].cmp(&b[j]);
            if !order.is_eq() {
                return order;
            }
            i += 1;
            j += 1;
        }
    }
    a.len().cmp(&b.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    pub(crate) fn zip(entries: &[(&str, &[u8])], deflate: bool) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for &(name, data) in entries {
            let packed = if deflate {
                let mut encoder =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(data).unwrap();
                encoder.finish().unwrap()
            } else {
                data.to_vec()
            };
            let method = if deflate { 8u16 } else { 0 };
            let offset = out.len() as u32;
            out.extend(0x0403_4b50u32.to_le_bytes());
            out.extend(20u16.to_le_bytes());
            out.extend(0x0800u16.to_le_bytes());
            out.extend(method.to_le_bytes());
            out.extend([0; 4]);
            out.extend(crc32fast::hash(data).to_le_bytes());
            out.extend((packed.len() as u32).to_le_bytes());
            out.extend((data.len() as u32).to_le_bytes());
            out.extend((name.len() as u16).to_le_bytes());
            out.extend([0; 2]);
            out.extend(name.as_bytes());
            out.extend(packed);
            directory.extend(0x0201_4b50u32.to_le_bytes());
            directory.extend(20u16.to_le_bytes());
            directory.extend(20u16.to_le_bytes());
            directory.extend(0x0800u16.to_le_bytes());
            directory.extend(method.to_le_bytes());
            directory.extend([0; 4]);
            directory.extend(crc32fast::hash(data).to_le_bytes());
            directory
                .extend(((out.len() - offset as usize - 30 - name.len()) as u32).to_le_bytes());
            directory.extend((data.len() as u32).to_le_bytes());
            directory.extend((name.len() as u16).to_le_bytes());
            directory.extend([0; 12]);
            directory.extend(offset.to_le_bytes());
            directory.extend(name.as_bytes());
        }
        let start = out.len() as u32;
        let len = directory.len() as u32;
        out.extend(directory);
        out.extend(0x0605_4b50u32.to_le_bytes());
        out.extend([0; 4]);
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend(len.to_le_bytes());
        out.extend(start.to_le_bytes());
        out.extend([0; 2]);
        out
    }
    pub(crate) fn pak(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = b"PACK".to_vec();
        out.extend([0; 8]);
        let mut directory = Vec::new();
        for &(name, data) in entries {
            let start = out.len() as i32;
            out.extend(data);
            let mut text = [0; 56];
            text[..name.len()].copy_from_slice(name.as_bytes());
            directory.extend(text);
            directory.extend(start.to_le_bytes());
            directory.extend((data.len() as i32).to_le_bytes());
        }
        let start = out.len() as i32;
        out[4..8].copy_from_slice(&start.to_le_bytes());
        out[8..12].copy_from_slice(&(directory.len() as i32).to_le_bytes());
        out.extend(directory);
        out
    }
    pub(crate) fn fixture(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("heron-asset-fs-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }
    #[test]
    fn mods_override_base_loose_files_and_loose_files_override_their_own_archives() {
        let root = fixture("priority");
        let base = root.join("base");
        let mod1 = root.join("mod1");
        let mod2 = root.join("mod2");
        for dir in [&base, &mod1, &mod2] {
            std::fs::create_dir_all(dir.join("textures")).unwrap();
        }
        std::fs::write(base.join("textures/a.png"), b"base loose").unwrap();
        std::fs::write(
            mod1.join("pak0.pak"),
            pak(&[("textures/a.png", b"mod pak")]),
        )
        .unwrap();
        let fs = AssetFileSystem::mount(&[base.clone(), mod1.clone()]);
        assert_eq!(fs.read("Textures\\a.png"), Some(b"mod pak".to_vec()));
        std::fs::write(mod1.join("textures/a.png"), b"mod loose").unwrap();
        std::fs::write(
            mod2.join("pak0.pk3"),
            zip(&[("textures/a.png", b"top compressed")], true),
        )
        .unwrap();
        let fs = AssetFileSystem::mount(&[base, mod1.clone(), mod2.clone()]);
        assert_eq!(fs.read("textures/a.png"), Some(b"top compressed".to_vec()));
        let fs = AssetFileSystem::mount(&[mod2.clone(), mod1, mod2]);
        assert_eq!(fs.read("textures/a.png"), Some(b"top compressed".to_vec()));
        assert!(fs.warnings().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn numbered_packages_use_natural_order_and_zip_can_store_or_deflate() {
        let root = fixture("numbering");
        std::fs::write(root.join("pak2.pak"), pak(&[("thing.txt", b"two")])).unwrap();
        std::fs::write(root.join("pak10.pak"), pak(&[("thing.txt", b"ten")])).unwrap();
        std::fs::write(
            root.join("extra.ZIP"),
            zip(&[("stored.txt", b"stored")], false),
        )
        .unwrap();
        let fs = AssetFileSystem::mount(std::slice::from_ref(&root));
        assert_eq!(fs.read("thing.txt"), Some(b"ten".to_vec()));
        assert_eq!(fs.read("stored.txt"), Some(b"stored".to_vec()));
        assert!(fs
            .resolve("thing.txt")
            .unwrap()
            .display_path()
            .to_string_lossy()
            .contains("pak10.pak!/thing.txt"));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn paths_are_bounded_case_insensitive_and_normalize_internal_parent_steps() {
        let root = fixture("paths");
        std::fs::create_dir_all(root.join("Models")).unwrap();
        std::fs::write(root.join("Models/Skin.PNG"), b"skin").unwrap();
        std::fs::write(
            root.join("bad.pk3"),
            zip(
                &[
                    ("../outside", b"no"),
                    ("/absolute", b"no"),
                    ("valid", b"yes"),
                ],
                false,
            ),
        )
        .unwrap();
        let fs = AssetFileSystem::mount(std::slice::from_ref(&root));
        assert_eq!(fs.read("models/part/../skin.png"), Some(b"skin".to_vec()));
        for name in [
            "../outside",
            "/absolute",
            "C:\\absolute",
            "models/../../outside",
        ] {
            assert!(fs.read(name).is_none(), "{name}");
        }
        assert_eq!(fs.read("valid"), Some(b"yes".to_vec()));
        assert_eq!(fs.warnings().len(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupt_zip_members_are_not_returned_and_bad_archives_do_not_hide_loose_assets() {
        let root = fixture("corrupt");
        let mut bytes = zip(&[("file", b"content")], false);
        bytes[34] ^= 1;
        std::fs::write(root.join("bad.zip"), bytes).unwrap();
        std::fs::write(root.join("bad.pak"), b"broken").unwrap();
        std::fs::write(root.join("loose"), b"okay").unwrap();
        let fs = AssetFileSystem::mount(std::slice::from_ref(&root));
        assert!(fs
            .resolve("file")
            .unwrap()
            .read()
            .unwrap_err()
            .contains("CRC"));
        assert_eq!(fs.read("loose"), Some(b"okay".to_vec()));
        assert!(!fs.warnings().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn hostile_zip_sizes_and_truncated_directories_fail_without_panics() {
        let original = zip(&[("asset", b"hello")], true);
        for len in 0..original.len() {
            let _ = zip::Zip::parse(original[..len].to_vec());
        }
        let mut bytes = original.clone();
        let central = bytes
            .windows(4)
            .position(|w| w == 0x0201_4b50u32.to_le_bytes())
            .unwrap();
        bytes[central + 24..central + 28].copy_from_slice(&u32::MAX.to_le_bytes());
        let parsed = zip::Zip::parse(bytes).unwrap();
        assert!(parsed.read(0).unwrap_err().contains("limit"));
        let mut bytes = original;
        bytes[central + 42..central + 46].copy_from_slice(&u32::MAX.to_le_bytes());
        let parsed = zip::Zip::parse(bytes).unwrap();
        assert!(parsed.read(0).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn directory_aliases_work_without_following_symlink_cycles() {
        let root = fixture("symlinks");
        std::fs::create_dir_all(root.join("original")).unwrap();
        std::fs::write(root.join("original/file"), b"yes").unwrap();
        std::os::unix::fs::symlink(root.join("original"), root.join("alias")).unwrap();
        std::os::unix::fs::symlink(&root, root.join("original/back")).unwrap();
        let fs = AssetFileSystem::mount(std::slice::from_ref(&root));
        assert_eq!(fs.read("alias/file"), Some(b"yes".to_vec()));
        assert_eq!(fs.read("original/file"), Some(b"yes".to_vec()));
        assert_eq!(fs.files().count(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "manual asset-index benchmark over 20000 archived resources"]
    fn profile_indexed_game_asset_lookup() {
        let root = fixture("benchmark");
        let names: Vec<_> = (0..20000).map(|i| format!("bulk/asset{i}.bin")).collect();
        let content = vec![42; 256];
        let entries: Vec<_> = names
            .iter()
            .map(|name| (name.as_str(), content.as_slice()))
            .collect();
        std::fs::write(root.join("pak0.pk3"), zip(&entries, true)).unwrap();
        let start = std::time::Instant::now();
        let fs = AssetFileSystem::mount(std::slice::from_ref(&root));
        let mount = start.elapsed();
        let start = std::time::Instant::now();
        for i in 0..200000 {
            std::hint::black_box(fs.resolve(&names[i % names.len()]).unwrap());
        }
        let lookups = start.elapsed();
        let start = std::time::Instant::now();
        for i in 0..20000 {
            assert_eq!(fs.read(&names[i]).unwrap(), content);
        }
        eprintln!(
            "asset-index: mount 20000 entries {mount:?}; 200000 indexed resolves {lookups:?}; 20000 bounded deflates {:?}",
            start.elapsed()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, target_os = "linux"))]
fn residency_root(kind: &str) -> PathBuf {
    for attempt in 0u64.. {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "heron-{kind}-residency-{}-{nonce}-{attempt}",
            std::process::id()
        ));
        match std::fs::create_dir(&root) {
            Ok(()) => return root,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("cannot create residency fixture: {error}"),
        }
    }
    unreachable!()
}

#[cfg(all(test, target_os = "linux"))]
mod residency_bench {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};
    fn rss_kib() -> u64 {
        std::fs::read_to_string("/proc/self/status")
            .unwrap()
            .lines()
            .find_map(|line| {
                line.strip_prefix("VmRSS:")
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(|n| n.parse().ok())
            })
            .unwrap()
    }
    #[test]
    #[ignore = "manual resident-memory benchmark for a generated sparse PAK"]
    fn bench_archive_residency() {
        let mb = std::env::var("HERON_BENCH_PAK_MB")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(256);
        assert!((1..=1024).contains(&mb));
        let root = residency_root("pak");
        let path = root.join("pak0.pak");
        let offset = mb * 1024 * 1024;
        {
            let mut file = std::fs::File::create(&path).unwrap();
            file.write_all(b"PACK").unwrap();
            file.write_all(&((offset + 5) as i32).to_le_bytes())
                .unwrap();
            file.write_all(&((((offset - 12).div_ceil(64 << 20) + 1) * 64) as i32).to_le_bytes())
                .unwrap();
            file.seek(SeekFrom::Start(offset)).unwrap();
            file.write_all(b"probe").unwrap();
            let mut name = [0; 56];
            name[..16].copy_from_slice(b"models/probe.bin");
            file.write_all(&name).unwrap();
            file.write_all(&(offset as i32).to_le_bytes()).unwrap();
            file.write_all(&5i32.to_le_bytes()).unwrap();
            let mut start = 12;
            let mut index = 0;
            while start < offset {
                let size = (offset - start).min(64 << 20);
                let mut name = [0; 56];
                let written = format!("unused/block{index}.bin");
                name[..written.len()].copy_from_slice(written.as_bytes());
                file.write_all(&name).unwrap();
                file.write_all(&(start as i32).to_le_bytes()).unwrap();
                file.write_all(&(size as i32).to_le_bytes()).unwrap();
                start += size;
                index += 1;
            }
        }
        let before = rss_kib();
        let started = std::time::Instant::now();
        let assets = AssetFileSystem::mount(std::slice::from_ref(&root));
        let elapsed = started.elapsed();
        assert!(assets.warnings().is_empty(), "{:?}", assets.warnings());
        assert_eq!(assets.read("models/probe.bin").unwrap(), b"probe");
        let after = rss_kib();
        let memory_mode = std::env::var("HERON_BENCH_PAK_MEMORY").as_deref() == Ok("1");
        // A generous fixture-specific bound catches retaining the whole payload.
        if !memory_mode && mb >= 64 {
            assert!(
                after.saturating_sub(before) < mb * 1024 / 2,
                "File-backed PAK mount retained too much memory: before {before} KiB, after {after} KiB, archive {mb} MiB"
            );
        }
        eprintln!(
            "PAK {mb} MiB (memory mode {}): mount {elapsed:?}; RSS before {before} KiB, mounted {after} KiB, delta {} KiB; {} indexed resources",
            memory_mode,
            after.saturating_sub(before),
            assets.files().count()
        );
        drop(assets);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, target_os = "linux"))]
mod zip_residency_bench {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};
    fn zero_crc(size: u64) -> u32 {
        let zero = [0; 65536];
        let mut crc = crc32fast::Hasher::new();
        let mut left = size;
        while left > 0 {
            let n = left.min(zero.len() as u64) as usize;
            crc.update(&zero[..n]);
            left -= n as u64;
        }
        crc.finalize()
    }
    fn rss_kib() -> u64 {
        std::fs::read_to_string("/proc/self/status")
            .unwrap()
            .lines()
            .find_map(|line| {
                line.strip_prefix("VmRSS:")
                    .and_then(|s| s.split_whitespace().next())
                    .and_then(|n| n.parse().ok())
            })
            .unwrap()
    }
    #[test]
    #[ignore = "manual resident-memory guard for a generated sparse ZIP/PK3"]
    fn bench_zip_archive_residency() {
        let mb = std::env::var("HERON_BENCH_ZIP_MB")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(256);
        assert!((1..=1024).contains(&mb));
        let root = residency_root("zip");
        let path = root.join("pak0.pk3");
        {
            let mut file = std::fs::File::create(&path).unwrap();
            let mut directory = Vec::new();
            let mut left = mb * 1024 * 1024;
            let mut count = 0u16;
            let full_crc = zero_crc(64 << 20);
            while left > 0 {
                let size = left.min(64 << 20);
                let name = format!("unused/block{count}.bin");
                let template = super::tests::zip(&[(&name, b"")], false);
                let at = u32::from_le_bytes(
                    template[template.len() - 6..template.len() - 2]
                        .try_into()
                        .unwrap(),
                ) as usize;
                let offset = file.stream_position().unwrap() as u32;
                let crc = if size == 64 << 20 {
                    full_crc
                } else {
                    zero_crc(size)
                };
                let mut header = template[..at].to_vec();
                header[14..18].copy_from_slice(&crc.to_le_bytes());
                header[18..22].copy_from_slice(&(size as u32).to_le_bytes());
                header[22..26].copy_from_slice(&(size as u32).to_le_bytes());
                file.write_all(&header).unwrap();
                file.seek(SeekFrom::Current(size as i64)).unwrap();
                let mut entry = template[at..template.len() - 22].to_vec();
                entry[16..20].copy_from_slice(&crc.to_le_bytes());
                entry[20..24].copy_from_slice(&(size as u32).to_le_bytes());
                entry[24..28].copy_from_slice(&(size as u32).to_le_bytes());
                entry[42..46].copy_from_slice(&offset.to_le_bytes());
                directory.extend(entry);
                left -= size;
                count += 1;
            }
            let probe = super::tests::zip(&[("models/probe.bin", b"probe")], true);
            let at = u32::from_le_bytes(probe[probe.len() - 6..probe.len() - 2].try_into().unwrap())
                as usize;
            let offset = file.stream_position().unwrap() as u32;
            file.write_all(&probe[..at]).unwrap();
            let mut entry = probe[at..probe.len() - 22].to_vec();
            entry[42..46].copy_from_slice(&offset.to_le_bytes());
            directory.extend(entry);
            count += 1;
            let offset = file.stream_position().unwrap() as u32;
            file.write_all(&directory).unwrap();
            file.write_all(&0x0605_4b50u32.to_le_bytes()).unwrap();
            file.write_all(&[0; 4]).unwrap();
            file.write_all(&count.to_le_bytes()).unwrap();
            file.write_all(&count.to_le_bytes()).unwrap();
            file.write_all(&(directory.len() as u32).to_le_bytes())
                .unwrap();
            file.write_all(&offset.to_le_bytes()).unwrap();
            file.write_all(&[0; 2]).unwrap();
        }
        let before = rss_kib();
        let started = std::time::Instant::now();
        let assets = AssetFileSystem::mount(std::slice::from_ref(&root));
        let mount = started.elapsed();
        assert!(assets.warnings().is_empty(), "{:?}", assets.warnings());
        assert_eq!(assets.read("models/probe.bin").unwrap(), b"probe");
        let after = rss_kib();
        let memory = std::env::var("HERON_BENCH_ZIP_MEMORY").as_deref() == Ok("1");
        if !memory && mb >= 64 {
            assert!(
                after.saturating_sub(before) < mb * 1024 / 2,
                "File-backed ZIP mount retained too much memory: before {before} KiB, after {after} KiB, archive {mb} MiB"
            );
        }
        eprintln!(
            "ZIP {mb} MiB (memory mode {memory}): mount {mount:?}; RSS before {before} KiB, mounted {after} KiB, delta {} KiB; {} indexed resources",
            after.saturating_sub(before),
            assets.files().count()
        );
        drop(assets);
        std::fs::remove_dir_all(root).unwrap();
    }
}
