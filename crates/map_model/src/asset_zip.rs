//! Stored/Deflate ZIP and PK3 members, addressed through the central directory.
use std::{borrow::Cow, io::Read};

const MAX_MEMBER: usize = 256 << 20;
pub(super) struct Zip {
    data: Data,
    entries: Vec<Entry>,
    directory: usize,
}
enum Data {
    Memory(file_store::SharedBytes),
    #[cfg(not(target_arch = "wasm32"))]
    File {
        file: std::sync::Mutex<std::fs::File>,
        len: usize,
        stamp: (u64, Option<std::time::SystemTime>),
    },
}

impl Data {
    fn len(&self) -> usize {
        match self {
            Self::Memory(bytes) => bytes.len(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::File { len, .. } => *len,
        }
    }
    fn range(&self, at: usize, length: usize) -> Result<Cow<'_, [u8]>, String> {
        let end = at
            .checked_add(length)
            .filter(|end| *end <= self.len())
            .ok_or("ZIP range exceeds archive bounds")?;
        match self {
            Self::Memory(bytes) => Ok(Cow::Borrowed(&bytes[at..end])),
            #[cfg(not(target_arch = "wasm32"))]
            Self::File { file, stamp, .. } => {
                use std::io::{Seek, SeekFrom};
                let mut file = file.lock().map_err(|_| "ZIP read lock is poisoned")?;
                let unchanged = |file: &std::fs::File| -> Result<(), String> {
                    let metadata = file.metadata().map_err(|e| e.to_string())?;
                    if (metadata.len(), metadata.modified().ok()) != *stamp {
                        return Err("ZIP/PK3 changed since mounting; reload game assets".into());
                    }
                    Ok(())
                };
                unchanged(&file)?;
                file.seek(SeekFrom::Start(at as u64))
                    .map_err(|e| e.to_string())?;
                let mut bytes = vec![0; length];
                file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
                unchanged(&file)?;
                Ok(Cow::Owned(bytes))
            }
        }
    }
}

struct Entry {
    name: String,
    method: u16,
    flags: u16,
    crc: u32,
    packed: usize,
    size: usize,
    local: usize,
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

impl Zip {
    #[cfg(test)]
    pub(super) fn parse(bytes: Vec<u8>) -> Result<Self, String> {
        Self::parse_shared(std::sync::Arc::new(bytes))
    }

    pub(super) fn parse_shared(bytes: file_store::SharedBytes) -> Result<Self, String> {
        Self::parse_data(Data::Memory(bytes))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn open(path: &std::path::Path) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        let len =
            usize::try_from(metadata.len()).map_err(|_| "ZIP is too large for this platform")?;
        Self::parse_data(Data::File {
            file: std::sync::Mutex::new(file),
            len,
            stamp: (metadata.len(), metadata.modified().ok()),
        })
    }

    fn parse_data(data: Data) -> Result<Self, String> {
        let tail_start = data.len().saturating_sub(65557);
        let tail = data.range(tail_start, data.len() - tail_start)?;
        let bytes = tail.as_ref();
        let end = bytes.len().checked_sub(22).ok_or("not a ZIP/PK3 archive")?;
        let eocd = (bytes.len().saturating_sub(65557)..=end)
            .rev()
            .find(|&at| {
                u32_at(bytes, at) == Some(0x0605_4b50)
                    && u16_at(bytes, at + 20).is_some_and(|n| at + 22 + n as usize == bytes.len())
            })
            .ok_or("missing ZIP central directory")?;
        let count = u16_at(bytes, eocd + 10).ok_or("truncated ZIP")? as usize;
        let length = u32_at(bytes, eocd + 12).ok_or("truncated ZIP")? as usize;
        let directory = u32_at(bytes, eocd + 16).ok_or("truncated ZIP")? as usize;
        if u16_at(bytes, eocd + 4) != Some(0)
            || u16_at(bytes, eocd + 6) != Some(0)
            || u16_at(bytes, eocd + 8) != Some(count as u16)
        {
            return Err("multi-disk ZIP is unsupported".into());
        }
        if count == 65535 || length == u32::MAX as usize || directory == u32::MAX as usize {
            return Err("ZIP64 is unsupported".into());
        }
        let limit = directory
            .checked_add(length)
            .filter(|&v| v <= tail_start + eocd)
            .ok_or("invalid ZIP directory bounds")?;
        if length > 64 << 20 {
            return Err("ZIP central directory exceeds 64 MiB".into());
        }
        drop(tail);
        let directory_bytes = data.range(directory, length)?;
        let bytes = directory_bytes.as_ref();
        let limit = limit - directory;
        let mut at = 0usize;
        let mut entries = Vec::new();
        for _ in 0..count {
            if at.checked_add(46).is_none_or(|end| end > limit)
                || u32_at(bytes, at) != Some(0x0201_4b50)
            {
                return Err("invalid ZIP directory entry".into());
            }
            let n = u16_at(bytes, at + 28).unwrap() as usize;
            let extra = u16_at(bytes, at + 30).unwrap() as usize;
            let comment = u16_at(bytes, at + 32).unwrap() as usize;
            let next = at
                .checked_add(46 + n + extra + comment)
                .filter(|&end| end <= limit)
                .ok_or("truncated ZIP name")?;
            if u16_at(bytes, at + 34) != Some(0) {
                return Err("multi-disk ZIP entry is unsupported".into());
            }
            entries.push(Entry {
                name: String::from_utf8_lossy(&bytes[at + 46..at + 46 + n]).into_owned(),
                flags: u16_at(bytes, at + 8).unwrap(),
                method: u16_at(bytes, at + 10).unwrap(),
                crc: u32_at(bytes, at + 16).unwrap(),
                packed: u32_at(bytes, at + 20).unwrap() as usize,
                size: u32_at(bytes, at + 24).unwrap() as usize,
                local: u32_at(bytes, at + 42).unwrap() as usize,
            });
            at = next;
        }
        if at != limit {
            return Err("ZIP directory size does not match its entries".into());
        }
        drop(directory_bytes);
        Ok(Self {
            data,
            entries,
            directory,
        })
    }
    pub(super) fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.name.as_str())
    }
    pub(super) fn read(&self, index: usize) -> Result<Vec<u8>, String> {
        let entry = self.entries.get(index).ok_or("missing ZIP entry")?;
        if entry.flags & 1 != 0 {
            return Err("encrypted ZIP entry is unsupported".into());
        }
        if entry.size > MAX_MEMBER || entry.packed > MAX_MEMBER {
            return Err("ZIP entry exceeds 256 MiB limit".into());
        }
        if !matches!(entry.method, 0 | 8) {
            return Err(format!(
                "unsupported ZIP compression method {}",
                entry.method
            ));
        }
        let at = entry.local;
        if at.checked_add(30).is_none_or(|end| end > self.directory) {
            return Err("invalid ZIP local header".into());
        }
        let header = self.data.range(at, 30)?;
        if u32_at(&header, 0) != Some(0x0403_4b50) {
            return Err("invalid ZIP local header".into());
        }
        if u16_at(&header, 8) != Some(entry.method) || u16_at(&header, 6) != Some(entry.flags) {
            return Err("ZIP headers disagree".into());
        }
        let n = u16_at(&header, 26).unwrap() as usize;
        let extra = u16_at(&header, 28).unwrap() as usize;
        let start = at
            .checked_add(30 + n + extra)
            .filter(|&end| end <= self.directory)
            .ok_or("invalid ZIP data offset")?;
        if n != entry.name.len() {
            return Err("ZIP local name differs from directory".into());
        }
        let end = start
            .checked_add(entry.packed)
            .filter(|&end| end <= self.directory)
            .ok_or("truncated ZIP entry")?;
        // Name, local extra fields and payload are contiguous. Native storage
        // reads them together, avoiding a separate seek/read and stamp checks
        // for the name. Memory storage still borrows the original buffer.
        let body = self.data.range(at + 30, end - (at + 30))?;
        if &body[..n] != entry.name.as_bytes() {
            return Err("ZIP local name differs from directory".into());
        }
        let prefix = n + extra;
        let result = match entry.method {
            0 => match body {
                Cow::Borrowed(bytes) => bytes[prefix..].to_vec(),
                Cow::Owned(mut bytes) => {
                    // Keep the native read allocation instead of allocating a
                    // second full stored-member buffer to remove the prefix.
                    bytes.copy_within(prefix.., 0);
                    bytes.truncate(entry.packed);
                    bytes
                }
            },
            8 => {
                let mut decoder = flate2::read::DeflateDecoder::new(&body[prefix..]);
                let mut result = Vec::new();
                decoder
                    .by_ref()
                    .take(entry.size as u64 + 1)
                    .read_to_end(&mut result)
                    .map_err(|e| e.to_string())?;
                if decoder.total_in() != entry.packed as u64 {
                    return Err("ZIP deflate size mismatch".into());
                }
                result
            }
            _ => {
                return Err(format!(
                    "unsupported ZIP compression method {}",
                    entry.method
                ));
            }
        };
        if result.len() != entry.size || crc32fast::hash(&result) != entry.crc {
            return Err("ZIP entry size or CRC mismatch".into());
        }
        Ok(result)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn shared_archives_keep_the_original_payload_for_stored_and_deflate() {
        for deflate in [false, true] {
            let bytes =
                std::sync::Arc::new(super::super::tests::zip(&[("item", b"payload")], deflate));
            let archive = Zip::parse_shared(bytes.clone()).unwrap();
            let Data::Memory(retained) = &archive.data else {
                panic!("expected memory archive")
            };
            assert!(std::sync::Arc::ptr_eq(&bytes, retained));
            drop(bytes);
            assert_eq!(archive.read(0).unwrap(), b"payload");
        }
    }

    #[test]
    fn combined_member_read_skips_large_local_extra_and_preserves_empty_payloads() {
        let path = fixture("extra-fields", b"");
        for deflate in [false, true] {
            for payload in [b"".as_slice(), b"actual payload".as_slice()] {
                let mut bytes = super::super::tests::zip(&[("item", payload)], deflate);
                let old_end = bytes.len() - 22;
                let directory = u32_at(&bytes, old_end + 16).unwrap();
                let extra = u16::MAX;
                bytes[28..30].copy_from_slice(&extra.to_le_bytes());
                bytes.splice(34..34, std::iter::repeat_n(0xa5, extra as usize));
                let end = bytes.len() - 22;
                bytes[end + 16..end + 20]
                    .copy_from_slice(&(directory + extra as u32).to_le_bytes());
                verify_storage_parity(&path, bytes.clone());
                assert_eq!(Zip::parse(bytes.clone()).unwrap().read(0).unwrap(), payload);
                assert_eq!(Zip::open(&path).unwrap().read(0).unwrap(), payload);
                bytes[30] ^= 1;
                std::fs::write(&path, &bytes).unwrap();
                assert!(Zip::open(&path)
                    .unwrap()
                    .read(0)
                    .unwrap_err()
                    .contains("name differs"));
                assert!(Zip::parse(bytes)
                    .unwrap()
                    .read(0)
                    .unwrap_err()
                    .contains("name differs"));
            }
        }
        std::fs::remove_file(path).unwrap();
    }

    fn fixture(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("heron-file-zip-{}-{name}.pk3", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path
    }
    fn verify_storage_parity(path: &std::path::Path, bytes: Vec<u8>) {
        std::fs::write(path, &bytes).unwrap();
        match (Zip::parse(bytes), Zip::open(path)) {
            (Ok(memory), Ok(file)) => {
                assert_eq!(
                    memory.names().collect::<Vec<_>>(),
                    file.names().collect::<Vec<_>>()
                );
                for index in 0..memory.entries.len() {
                    assert_eq!(memory.read(index), file.read(index));
                }
            }
            (Err(a), Err(b)) => assert_eq!(a, b),
            _ => panic!("Memory and file ZIP readers disagree"),
        }
    }
    #[test]
    fn stored_deflate_truncations_corruption_and_max_comment_have_storage_parity() {
        let path = fixture("parity", b"");
        for deflate in [false, true] {
            let source = super::super::tests::zip(
                &[
                    ("one.bin", b"first payload"),
                    ("two.bin", b"second payload"),
                ],
                deflate,
            );
            verify_storage_parity(&path, source.clone());
            for end in 0..source.len() {
                verify_storage_parity(&path, source[..end].to_vec());
            }
            for at in 0..source.len() {
                let mut changed = source.clone();
                changed[at] ^= 0xff;
                verify_storage_parity(&path, changed);
            }
            let mut comment = source;
            let end = comment.len();
            comment[end - 2..].copy_from_slice(&u16::MAX.to_le_bytes());
            comment.extend(vec![b'a'; 65535]);
            verify_storage_parity(&path, comment);
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn native_zip_reads_reject_crc_bad_deflate_and_unsupported_flags_and_sizes() {
        let path = fixture("validation", b"");
        for deflate in [false, true] {
            let mut bytes = super::super::tests::zip(&[("item", b"payload")], deflate);
            bytes[34] ^= 0xff;
            std::fs::write(&path, &bytes).unwrap();
            assert!(Zip::open(&path).unwrap().read(0).is_err());
        }
        for (field, value, expected) in [
            (8, 1u32, "encrypted"),
            (20, u32::MAX, "256 MiB"),
            (24, u32::MAX, "256 MiB"),
        ] {
            let mut bytes = super::super::tests::zip(&[("item", b"payload")], false);
            let directory = u32_at(&bytes, bytes.len() - 6).unwrap() as usize;
            if field == 8 {
                bytes[directory + field..directory + field + 2]
                    .copy_from_slice(&(value as u16).to_le_bytes());
            } else {
                bytes[directory + field..directory + field + 4]
                    .copy_from_slice(&value.to_le_bytes());
            }
            std::fs::write(&path, &bytes).unwrap();
            assert!(Zip::open(&path)
                .unwrap()
                .read(0)
                .unwrap_err()
                .contains(expected));
        }
        let mut unsupported = super::super::tests::zip(&[("item", b"payload")], false);
        let directory = u32_at(&unsupported, unsupported.len() - 6).unwrap() as usize;
        unsupported[8..10].copy_from_slice(&93u16.to_le_bytes());
        unsupported[directory + 10..directory + 12].copy_from_slice(&93u16.to_le_bytes());
        unsupported[directory + 20..directory + 24].copy_from_slice(&(128u32 << 20).to_le_bytes());
        std::fs::write(&path, unsupported).unwrap();
        assert!(Zip::open(&path)
            .unwrap()
            .read(0)
            .unwrap_err()
            .contains("unsupported ZIP compression method 93"));
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn concurrent_zip_reads_keep_each_local_header_and_payload_together() {
        let payloads = [vec![13u8; 4096], vec![71u8; 509]];
        let path = fixture(
            "parallel",
            &super::super::tests::zip(&[("one", &payloads[0]), ("two", &payloads[1])], true),
        );
        let archive = std::sync::Arc::new(Zip::open(&path).unwrap());
        std::thread::scope(|scope| {
            for thread in 0..8 {
                let archive = archive.clone();
                let payloads = &payloads;
                scope.spawn(move || {
                    for sample in 0..100 {
                        let index = (sample + thread) % 2;
                        assert_eq!(archive.read(index).unwrap(), payloads[index]);
                    }
                });
            }
        });
        drop(archive);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn in_place_zip_changes_require_reload_and_reload_validates_new_crc() {
        use std::io::{Seek, SeekFrom, Write};
        let bytes = super::super::tests::zip(&[("item", b"old")], false);
        let path = fixture("change", &bytes);
        let archive = Zip::open(&path).unwrap();
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.seek(SeekFrom::Start(34)).unwrap();
        file.write_all(b"X").unwrap();
        file.set_modified(modified + std::time::Duration::from_secs(60))
            .unwrap();
        assert!(archive.read(0).unwrap_err().contains("changed"));
        assert!(Zip::open(&path)
            .unwrap()
            .read(0)
            .unwrap_err()
            .contains("CRC"));
        let valid = super::super::tests::zip(&[("item", b"new")], false);
        std::fs::write(&path, valid).unwrap();
        let reloaded = Zip::open(&path).unwrap();
        assert_eq!(reloaded.read(0).unwrap(), b"new");
        file.set_len(4).unwrap();
        assert!(reloaded.read(0).unwrap_err().contains("changed"));
        drop((archive, reloaded, file));
        std::fs::remove_file(path).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn zip_atomic_replacement_preserves_mounted_handle_and_large_directory_is_bounded() {
        use std::io::{Seek, SeekFrom, Write};
        let path = fixture(
            "replace",
            &super::super::tests::zip(&[("item", b"old")], true),
        );
        let old = Zip::open(&path).unwrap();
        let replacement = fixture(
            "replacement",
            &super::super::tests::zip(&[("item", b"new")], true),
        );
        std::fs::rename(replacement, &path).unwrap();
        assert_eq!(old.read(0).unwrap(), b"old");
        assert_eq!(Zip::open(&path).unwrap().read(0).unwrap(), b"new");
        drop(old);
        let mut file = std::fs::File::create(&path).unwrap();
        let length = (64u32 << 20) + 1;
        file.seek(SeekFrom::Start(length as u64)).unwrap();
        file.write_all(&0x0605_4b50u32.to_le_bytes()).unwrap();
        file.write_all(&[0; 4]).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&length.to_le_bytes()).unwrap();
        file.write_all(&[0; 6]).unwrap();
        assert!(Zip::open(&path).err().unwrap().contains("64 MiB"));
        drop(file);
        std::fs::remove_file(path).unwrap();
    }
}
