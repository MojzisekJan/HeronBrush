//! Reading a Quake `.pak`, which is where a game keeps its own files.
//!
//! Resolves model, skin and palette resources from loose files or game `.pak` archives.
//!
//! The format is a header -- `PACK`, then where the directory is and how long -- and one
//! sixty-four byte entry per file: a name, an offset and a size. There is no compression.

use std::path::Path;

/// One file inside a pak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its name, with `/` separators, as the game names it.
    pub name: String,
    /// Where it starts.
    offset: usize,
    /// How long it is.
    length: usize,
}

/// A pak that has been opened.
#[derive(Debug, Clone)]
pub struct Pak {
    bytes: file_store::SharedBytes,
    entries: Vec<Entry>,
}

/// Why a pak could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PakError {
    /// The file does not begin with `PACK`.
    NotAPak,
    /// The directory runs past the end of the file.
    Truncated,
}

impl std::fmt::Display for PakError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PakError::NotAPak => write!(out, "not a pak"),
            PakError::Truncated => write!(out, "the pak's directory ends early"),
        }
    }
}

impl std::error::Error for PakError {}

impl Pak {
    /// Opens a pak from a file.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Result<Self, PakError>> {
        Ok(Self::read(std::fs::read(path.as_ref())?))
    }

    /// Reads a pak that is already in memory.
    pub fn read(bytes: Vec<u8>) -> Result<Self, PakError> {
        Self::read_shared(std::sync::Arc::new(bytes))
    }

    /// Indexes an immutable snapshot without copying its archive payload.
    pub fn read_shared(bytes: file_store::SharedBytes) -> Result<Self, PakError> {
        if bytes.len() < 12 || &bytes[..4] != b"PACK" {
            return Err(PakError::NotAPak);
        }
        // A pak writes every offset and size as a *signed* 32-bit number, and a
        // corrupt or hostile file can hold a negative one. Cast straight to `usize`
        // that becomes a value near 2^64, which then overflows any bound check added
        // after it and comes out the other side looking small and safe. So a negative
        // is refused here, where it is read, rather than guarded against everywhere it
        // is used.
        let number = |at: usize| -> Option<usize> {
            usize::try_from(i32::from_le_bytes([
                bytes[at],
                bytes[at + 1],
                bytes[at + 2],
                bytes[at + 3],
            ]))
            .ok()
        };
        let (Some(directory), Some(length)) = (number(4), number(8)) else {
            return Err(PakError::Truncated);
        };
        if length % 64 != 0
            || directory
                .checked_add(length)
                .is_none_or(|end| end > bytes.len())
        {
            return Err(PakError::Truncated);
        }

        let entries = parse_entries(&bytes[directory..directory + length], bytes.len());
        Ok(Self { bytes, entries })
    }

    /// What is in it.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// One file by its directory index, without a linear name search.
    pub fn read_index(&self, index: usize) -> Option<&[u8]> {
        let entry = self.entries.get(index)?;
        self.bytes
            .get(entry.offset..entry.offset.checked_add(entry.length)?)
    }

    /// One file's contents, by the name the game knows it as.
    ///
    /// The comparison ignores case and takes `\` for `/`, since a pak written on one
    /// system is read on another.
    pub fn read_file(&self, name: &str) -> Option<&[u8]> {
        let wanted = tidy(name);
        self.entries
            .iter()
            .find(|entry| tidy(&entry.name) == wanted)
            .map(|entry| &self.bytes[entry.offset..entry.offset + entry.length])
    }
}

fn parse_entries(bytes: &[u8], file_length: usize) -> Vec<Entry> {
    let number =
        |at: usize| usize::try_from(i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())).ok();
    let mut entries = Vec::with_capacity(bytes.len() / 64);
    for at in (0..bytes.len()).step_by(64) {
        if at + 64 > bytes.len() {
            break;
        }
        let name = bytes[at..at + 56]
            .iter()
            .take_while(|byte| **byte != 0)
            .map(|byte| char::from(*byte))
            .collect::<String>();
        let (Some(offset), Some(size)) = (number(at + 56), number(at + 60)) else {
            continue;
        };
        // Checked, since these are two numbers the file chose.
        if offset.checked_add(size).is_none_or(|end| end > file_length) {
            continue;
        }
        entries.push(Entry {
            name,
            offset,
            length: size,
        });
    }
    entries
}

/// Shared resolver limit for a requested PAK member, matching ZIP's decoded limit.
pub const MAX_MEMBER_BYTES: usize = 256 << 20;

/// A native PAK index that retains an open file rather than the archive payload.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct FilePak {
    file: std::sync::Mutex<std::fs::File>,
    stamp: (u64, Option<std::time::SystemTime>),
    entries: Vec<Entry>,
}

#[cfg(not(target_arch = "wasm32"))]
impl FilePak {
    /// Reads only the header and directory. File replacements retain the mounted
    /// handle; in-place length/mtime changes are rejected until assets are reloaded.
    pub fn open(path: &Path) -> Result<Self, String> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        let stamp = (metadata.len(), metadata.modified().ok());
        let file_length =
            usize::try_from(metadata.len()).map_err(|_| "PAK is too large for this platform")?;
        let mut header = [0; 12];
        file.read_exact(&mut header).map_err(|e| e.to_string())?;
        if &header[..4] != b"PACK" {
            return Err(PakError::NotAPak.to_string());
        }
        let number = |at| {
            usize::try_from(i32::from_le_bytes(header[at..at + 4].try_into().unwrap()))
                .map_err(|_| PakError::Truncated.to_string())
        };
        let (offset, length) = (number(4)?, number(8)?);
        if length % 64 != 0
            || offset
                .checked_add(length)
                .is_none_or(|end| end > file_length)
        {
            return Err(PakError::Truncated.to_string());
        }
        if length > 64 << 20 {
            return Err("PAK directory exceeds 64 MiB".into());
        }
        file.seek(SeekFrom::Start(offset as u64))
            .map_err(|e| e.to_string())?;
        let mut directory = vec![0; length];
        file.read_exact(&mut directory).map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if (metadata.len(), metadata.modified().ok()) != stamp {
            return Err("PAK changed while mounting; reload game assets".into());
        }
        Ok(Self {
            file: std::sync::Mutex::new(file),
            stamp,
            entries: parse_entries(&directory, file_length),
        })
    }

    /// Validated members in directory order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Copies only the requested member, with bounded allocation and serialized seeks.
    pub fn read_index(&self, index: usize) -> Result<Vec<u8>, String> {
        use std::io::{Read, Seek, SeekFrom};
        let entry = self.entries.get(index).ok_or("missing PAK entry")?;
        if entry.length > MAX_MEMBER_BYTES {
            return Err("PAK member exceeds 256 MiB".into());
        }
        let mut file = self.file.lock().map_err(|_| "PAK read lock is poisoned")?;
        let unchanged = |file: &std::fs::File| -> Result<(), String> {
            let metadata = file.metadata().map_err(|e| e.to_string())?;
            if (metadata.len(), metadata.modified().ok()) != self.stamp {
                return Err("PAK changed since mounting; reload game assets".into());
            }
            Ok(())
        };
        unchanged(&file)?;
        file.seek(SeekFrom::Start(entry.offset as u64))
            .map_err(|e| e.to_string())?;
        let mut bytes = vec![0; entry.length];
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        unchanged(&file)?;
        Ok(bytes)
    }
}

/// A name in the one shape names are compared in.
fn tidy(name: &str) -> String {
    name.replace('\\', "/").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_archive_and_clone_keep_the_original_payload() {
        let bytes = std::sync::Arc::new(one_file());
        let pak = Pak::read_shared(bytes.clone()).unwrap();
        assert!(std::sync::Arc::ptr_eq(&bytes, &pak.bytes));
        let cloned = pak.clone();
        assert!(std::sync::Arc::ptr_eq(&pak.bytes, &cloned.bytes));
        drop((bytes, pak));
        assert_eq!(
            cloned.read_file("progs/armor.mdl"),
            Some(&b"a model, more or less"[..])
        );
    }

    /// A pak holding one file called `progs/armor.mdl`.
    fn one_file() -> Vec<u8> {
        let contents = b"a model, more or less";
        let mut out = Vec::new();
        out.extend_from_slice(b"PACK");
        out.extend_from_slice(&(12i32 + contents.len() as i32).to_le_bytes());
        out.extend_from_slice(&64i32.to_le_bytes());
        out.extend_from_slice(contents);

        let mut name = [0u8; 56];
        name[..16].copy_from_slice(b"progs/armor.mdl\0");
        out.extend_from_slice(&name);
        out.extend_from_slice(&12i32.to_le_bytes());
        out.extend_from_slice(&(contents.len() as i32).to_le_bytes());
        out
    }

    #[test]
    fn a_file_comes_back_by_the_name_the_game_knows_it_as() {
        let pak = Pak::read(one_file()).expect("it reads");
        assert_eq!(pak.entries().len(), 1);
        assert_eq!(
            pak.read_file("progs/armor.mdl"),
            Some(&b"a model, more or less"[..])
        );
        // A name written the other way round, or in another case, is the same name.
        assert!(pak.read_file("PROGS\\ARMOR.MDL").is_some());
        assert_eq!(pak.read_file("progs/nothing.mdl"), None);
    }

    #[test]
    fn something_that_is_not_a_pak_is_an_error() {
        assert_eq!(Pak::read(b"nope".to_vec()).err(), Some(PakError::NotAPak));
    }

    #[test]
    fn a_directory_past_the_end_is_an_error_rather_than_a_read_past_it() {
        let mut pak = one_file();
        // The directory says it runs a megabyte, which the file does not have.
        pak[8..12].copy_from_slice(&(1024i32 * 1024).to_le_bytes());
        assert_eq!(Pak::read(pak).err(), Some(PakError::Truncated));
    }

    /// A pak's numbers are signed, and a file can hold negative ones. Cast to `usize` a
    /// negative offset is enormous, and `offset + size` on it wrapped back into range --
    /// so the entry passed the bounds check and `read_file` then sliced the file with
    /// it. That panicked on a file the editor reads from the user's game directory. The
    /// addition itself panicked first in a debug build, which is how this is caught here.
    #[test]
    fn an_entry_with_a_negative_offset_is_skipped_rather_than_trusted() {
        for at in [56, 60] {
            let mut pak = one_file();
            let entry = pak.len() - 64;
            pak[entry + at..entry + at + 4].copy_from_slice(&(-1i32).to_le_bytes());
            let pak = Pak::read(pak).expect("the header is still good");
            assert!(
                pak.entries().is_empty(),
                "the entry at {at} should be refused"
            );
            assert_eq!(pak.read_file("progs/armor.mdl"), None);
        }
    }

    #[test]
    fn a_negative_number_in_the_header_is_an_error() {
        for at in [4, 8] {
            let mut pak = one_file();
            pak[at..at + 4].copy_from_slice(&(-1i32).to_le_bytes());
            assert_eq!(Pak::read(pak).err(), Some(PakError::Truncated), "at {at}");
        }
    }

    #[test]
    fn an_entry_reaching_past_the_end_of_the_file_is_skipped() {
        let mut pak = one_file();
        let entry = pak.len() - 64;
        pak[entry + 60..entry + 64].copy_from_slice(&i32::MAX.to_le_bytes());
        let pak = Pak::read(pak).expect("the header is still good");
        assert!(pak.entries().is_empty());
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn disk_fixture(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("heron-file-pak-{}-{name}.pak", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn file_and_memory_directories_agree_on_truncations_and_bad_ranges() {
        let original = one_file();
        let path = disk_fixture("parity", &original);
        let verify = |bytes: Vec<u8>| {
            std::fs::write(&path, &bytes).unwrap();
            let memory = Pak::read(bytes);
            let file = FilePak::open(&path);
            assert_eq!(memory.is_ok(), file.is_ok());
            if let (Ok(memory), Ok(file)) = (memory, file) {
                assert_eq!(memory.entries(), file.entries());
                for index in 0..memory.entries().len() {
                    assert_eq!(
                        memory.read_index(index).unwrap(),
                        file.read_index(index).unwrap()
                    );
                }
            }
        };
        for end in 0..=original.len() {
            verify(original[..end].to_vec());
        }
        for at in [4, 8, original.len() - 8, original.len() - 4] {
            let mut bytes = original.clone();
            bytes[at..at + 4].copy_from_slice(&(-1i32).to_le_bytes());
            verify(bytes);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn file_reads_reject_in_place_changes_and_reload_observes_new_bytes() {
        use std::io::{Seek, SeekFrom, Write};
        let path = disk_fixture("changed", &one_file());
        let reader = FilePak::open(&path).unwrap();
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        let mut writer = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        writer.seek(SeekFrom::Start(12)).unwrap();
        writer.write_all(b"X").unwrap();
        writer
            .set_modified(modified + std::time::Duration::from_secs(60))
            .unwrap();
        assert!(reader.read_index(0).unwrap_err().contains("changed"));
        let reloaded = FilePak::open(&path).unwrap();
        assert_eq!(reloaded.read_index(0).unwrap()[0], b'X');
        writer.set_len(12).unwrap();
        assert!(reloaded.read_index(0).unwrap_err().contains("changed"));
        drop((writer, reader, reloaded));
        std::fs::remove_file(path).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn concurrent_file_members_do_not_share_an_unprotected_seek_cursor() {
        let payloads = [vec![17u8; 4096], vec![93u8; 257]];
        let mut bytes = b"PACK".to_vec();
        bytes.extend([0; 8]);
        let mut directory = Vec::new();
        for (i, payload) in payloads.iter().enumerate() {
            let mut name = [0; 56];
            name[0] = b'a' + i as u8;
            directory.extend(name);
            directory.extend((bytes.len() as i32).to_le_bytes());
            directory.extend((payload.len() as i32).to_le_bytes());
            bytes.extend(payload);
        }
        let at = bytes.len() as i32;
        bytes[4..8].copy_from_slice(&at.to_le_bytes());
        bytes[8..12].copy_from_slice(&128i32.to_le_bytes());
        bytes.extend(directory);
        let path = disk_fixture("parallel", &bytes);
        let reader = std::sync::Arc::new(FilePak::open(&path).unwrap());
        std::thread::scope(|scope| {
            for thread in 0..8 {
                let reader = reader.clone();
                let payloads = &payloads;
                scope.spawn(move || {
                    for sample in 0..100 {
                        let index = (sample + thread) % 2;
                        assert_eq!(reader.read_index(index).unwrap(), payloads[index]);
                    }
                });
            }
        });
        assert!(reader.read_index(99).unwrap_err().contains("missing"));
        drop(reader);
        std::fs::remove_file(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn replacing_a_path_keeps_the_mounted_handle_until_reload() {
        let original = one_file();
        let path = disk_fixture("replacement", &original);
        let old = FilePak::open(&path).unwrap();
        let mut changed = original.clone();
        changed[12] = b'Z';
        let replacement = disk_fixture("replacement-new", &changed);
        std::fs::rename(replacement, &path).unwrap();
        assert_eq!(old.read_index(0).unwrap()[0], original[12]);
        assert_eq!(
            FilePak::open(&path).unwrap().read_index(0).unwrap()[0],
            b'Z'
        );
        drop(old);
        std::fs::remove_file(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn file_member_and_directory_limits_reject_sparse_oversized_inputs() {
        use std::io::{Seek, SeekFrom, Write};
        let path = disk_fixture("limits", b"");
        let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        let length = (MAX_MEMBER_BYTES + 1) as i32;
        let directory = 12 + length;
        file.write_all(b"PACK").unwrap();
        file.write_all(&directory.to_le_bytes()).unwrap();
        file.write_all(&64i32.to_le_bytes()).unwrap();
        file.seek(SeekFrom::Start(directory as u64)).unwrap();
        let mut name = [0; 56];
        name[0] = b'a';
        file.write_all(&name).unwrap();
        file.write_all(&12i32.to_le_bytes()).unwrap();
        file.write_all(&length.to_le_bytes()).unwrap();
        let reader = FilePak::open(&path).unwrap();
        assert!(reader.read_index(0).unwrap_err().contains("256 MiB"));
        drop(reader);
        file.set_len(12 + ((64 << 20) + 64)).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(b"PACK").unwrap();
        file.write_all(&12i32.to_le_bytes()).unwrap();
        file.write_all(&((64i32 << 20) + 64).to_le_bytes()).unwrap();
        assert!(FilePak::open(&path)
            .unwrap_err()
            .contains("directory exceeds"));
        drop(file);
        std::fs::remove_file(path).unwrap();
    }
}
