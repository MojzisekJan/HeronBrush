//! Reading a Quake `.pak`, which is where a game keeps its own files.
//!
//! Resolves model, skin and palette resources from loose files or game `.pak` archives.
//!
//! The format is a header -- `PACK`, then where the directory is and how long -- and one
//! sixty-four byte entry per file: a name, an offset and a size. There is no compression.
//!
//! Daikatana's paks (TrenchBroom calls them `dkpak`) have the same header and
//! seventy-two byte entries: the name, offset and size, then the stored size and a flag
//! saying the file is compressed with Ion Storm's run-length and back-reference scheme.
//! Which of the two a directory holds is read from the directory itself.

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
    /// How long it is in the pak, if it is stored compressed (Daikatana).
    stored: Option<usize>,
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
        if !fits_either_layout(length)
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
    ///
    /// A compressed file comes back expanded, so owned; a plain one is borrowed.
    pub fn read_index(&self, index: usize) -> Option<std::borrow::Cow<'_, [u8]>> {
        let entry = self.entries.get(index)?;
        let stored = self
            .bytes
            .get(entry.offset..entry.offset.checked_add(entry.stored_length())?)?;
        match entry.stored {
            None => Some(std::borrow::Cow::Borrowed(stored)),
            Some(_) => expand(stored, entry.length)
                .ok()
                .map(std::borrow::Cow::Owned),
        }
    }

    /// One file's contents, by the name the game knows it as.
    ///
    /// The comparison ignores case and takes `\` for `/`, since a pak written on one
    /// system is read on another.
    pub fn read_file(&self, name: &str) -> Option<std::borrow::Cow<'_, [u8]>> {
        let wanted = tidy(name);
        let index = self
            .entries
            .iter()
            .position(|entry| tidy(&entry.name) == wanted)?;
        self.read_index(index)
    }
}

impl Entry {
    /// How many bytes it takes up in the pak.
    fn stored_length(&self) -> usize {
        self.stored.unwrap_or(self.length)
    }
}

/// Quake's directory entry, and Daikatana's with its stored size and compression flag.
const QUAKE_ENTRY: usize = 64;
const DAIKATANA_ENTRY: usize = 72;

/// Whether a directory of this many bytes holds whole entries of either kind.
fn fits_either_layout(length: usize) -> bool {
    length.is_multiple_of(QUAKE_ENTRY) || length.is_multiple_of(DAIKATANA_ENTRY)
}

/// How long each entry in this directory is.
///
/// A length that only one size divides settles it. One that both divide (576 bytes is
/// nine Quake entries or eight Daikatana ones) is read as Quake's, unless its names are
/// not names when read that way and Daikatana's reading gives names and flags that are.
/// Read at the wrong size, the second entry's name starts inside the first entry's
/// numbers, which are not printable text.
fn entry_size(directory: &[u8]) -> usize {
    let length = directory.len();
    if !length.is_multiple_of(DAIKATANA_ENTRY) {
        return QUAKE_ENTRY;
    }
    if !length.is_multiple_of(QUAKE_ENTRY) {
        return DAIKATANA_ENTRY;
    }
    let plausible = |size: usize| {
        directory.chunks_exact(size).all(|entry| {
            let name = entry[..56].split(|byte| *byte == 0).next().unwrap_or(&[]);
            let flag_is_flag = size == QUAKE_ENTRY || matches!(entry[68..72], [0 | 1, 0, 0, 0]);
            !name.is_empty() && name.iter().all(|byte| (0x20..0x7f).contains(byte)) && flag_is_flag
        })
    };
    if !plausible(QUAKE_ENTRY) && plausible(DAIKATANA_ENTRY) {
        DAIKATANA_ENTRY
    } else {
        QUAKE_ENTRY
    }
}

fn parse_entries(bytes: &[u8], file_length: usize) -> Vec<Entry> {
    let number = |at: usize| {
        usize::try_from(i32::from_le_bytes(
            bytes[at..at + 4].try_into().expect("four bytes"),
        ))
        .ok()
    };
    let size_of_entry = entry_size(bytes);
    let mut entries = Vec::with_capacity(bytes.len() / size_of_entry);
    for at in (0..bytes.len()).step_by(size_of_entry) {
        if at + size_of_entry > bytes.len() {
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
        let stored = if size_of_entry == DAIKATANA_ENTRY && number(at + 68) != Some(0) {
            let Some(stored) = number(at + 64) else {
                continue;
            };
            Some(stored)
        } else {
            None
        };
        let entry = Entry {
            name,
            offset,
            length: size,
            stored,
        };
        // Checked, since these are numbers the file chose.
        if offset
            .checked_add(entry.stored_length())
            .is_none_or(|end| end > file_length)
        {
            continue;
        }
        entries.push(entry);
    }
    entries
}

/// Expands a file Daikatana's `dkpak` compressed, which is `length` bytes once expanded.
///
/// The stream is a run of commands, each one byte, as yquake2's `pakextract` reads them:
/// below 64 copies the next `n + 1` bytes, below 128 writes `n - 62` zeros, below 192
/// writes the next byte `n - 126` times, below 254 copies `n - 190` bytes from `b + 2`
/// back in what is already written (`b` is the next byte, and the copy may run into
/// itself), 254 does nothing and 255 ends it.
fn expand(stored: &[u8], length: usize) -> Result<Vec<u8>, String> {
    if length > MAX_MEMBER_BYTES {
        return Err("PAK member exceeds 256 MiB".into());
    }
    let broken = || "a compressed PAK member does not expand to its size".to_string();
    let mut out = Vec::with_capacity(length);
    let mut at = 0;
    while let Some(&command) = stored.get(at) {
        at += 1;
        let next = |at: usize| stored.get(at).copied().ok_or_else(broken);
        let run = match command {
            0..64 => usize::from(command) + 1,
            64..128 => usize::from(command) - 62,
            128..192 => usize::from(command) - 126,
            192..254 => usize::from(command) - 190,
            254 => continue,
            255 => break,
        };
        if out.len() + run > length {
            return Err(broken());
        }
        match command {
            0..64 => {
                out.extend_from_slice(stored.get(at..at + run).ok_or_else(broken)?);
                at += run;
            }
            64..128 => out.resize(out.len() + run, 0),
            128..192 => {
                let byte = next(at)?;
                at += 1;
                out.resize(out.len() + run, byte);
            }
            _ => {
                let back = usize::from(next(at)?) + 2;
                at += 1;
                let start = out.len().checked_sub(back).ok_or_else(broken)?;
                for from in start..start + run {
                    out.push(out[from]);
                }
            }
        }
    }
    if out.len() == length {
        Ok(out)
    } else {
        Err(broken())
    }
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
        if !fits_either_layout(length)
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
        if entry.length.max(entry.stored_length()) > MAX_MEMBER_BYTES {
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
        let mut bytes = vec![0; entry.stored_length()];
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        unchanged(&file)?;
        match entry.stored {
            None => Ok(bytes),
            Some(_) => expand(&bytes, entry.length),
        }
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
            cloned.read_file("progs/armor.mdl").as_deref(),
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
            pak.read_file("progs/armor.mdl").as_deref(),
            Some(&b"a model, more or less"[..])
        );
        // A name written the other way round, or in another case, is the same name.
        assert!(pak.read_file("PROGS\\ARMOR.MDL").is_some());
        assert!(pak.read_file("progs/nothing.mdl").is_none());
    }

    /// A Daikatana stream using every command, and what it expands to.
    const DK_STREAM: &[u8] = &[
        2, b'a', b'b', b'c', // three bytes as they are
        192, 1,  // two bytes from three back: "ab"
        65, // three zeros
        130, b'z', // four of 'z'
        193, 0,    // three bytes from two back, running into what it writes
        254,  // nothing
        255,  // the end
        b'!', // never read
    ];
    const DK_EXPANDED: &[u8] = b"abcab\0\0\0zzzzzzz";

    /// A Daikatana pak holding `maps/plain.txt` stored as it is and `maps/packed.txt`
    /// compressed.
    fn daikatana_pak() -> Vec<u8> {
        let plain = b"stored as it is";
        let mut out = b"PACK".to_vec();
        out.extend([0; 8]);
        let mut directory = Vec::new();
        for (name, stored, length, compressed) in [
            (&b"maps/plain.txt"[..], &plain[..], plain.len(), 0i32),
            (&b"maps/packed.txt"[..], DK_STREAM, DK_EXPANDED.len(), 1),
        ] {
            let mut field = [0u8; 56];
            field[..name.len()].copy_from_slice(name);
            directory.extend(field);
            directory.extend((out.len() as i32).to_le_bytes());
            directory.extend((length as i32).to_le_bytes());
            directory.extend((stored.len() as i32).to_le_bytes());
            directory.extend(compressed.to_le_bytes());
            out.extend(stored);
        }
        let at = out.len() as i32;
        out[4..8].copy_from_slice(&at.to_le_bytes());
        out[8..12].copy_from_slice(&(directory.len() as i32).to_le_bytes());
        out.extend(directory);
        out
    }

    #[test]
    fn a_daikatana_pak_reads_stored_and_compressed_files() {
        let pak = Pak::read(daikatana_pak()).expect("a Daikatana pak reads");
        let names: Vec<_> = pak.entries().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["maps/plain.txt", "maps/packed.txt"]);
        assert_eq!(
            pak.read_file("maps/plain.txt").as_deref(),
            Some(&b"stored as it is"[..])
        );
        assert_eq!(
            pak.read_file("MAPS\\PACKED.TXT").as_deref(),
            Some(DK_EXPANDED)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_daikatana_pak_reads_the_same_from_disk() {
        let bytes = daikatana_pak();
        let path = disk_fixture("daikatana", &bytes);
        let file = FilePak::open(&path).expect("the directory reads");
        let memory = Pak::read(bytes).expect("a Daikatana pak reads");
        assert_eq!(file.entries(), memory.entries());
        assert_eq!(file.read_index(1).expect("it expands"), DK_EXPANDED);
        std::fs::remove_file(path).expect("the fixture is removed");
    }

    #[test]
    fn a_compressed_stream_that_does_not_add_up_is_refused() {
        assert_eq!(
            expand(DK_STREAM, DK_EXPANDED.len()).as_deref(),
            Ok(DK_EXPANDED)
        );
        // Short of its size, past it, cut off in a literal or a run, or reaching back
        // before the start.
        for (stream, length) in [
            (DK_STREAM, DK_EXPANDED.len() + 1),
            (DK_STREAM, DK_EXPANDED.len() - 1),
            (&DK_STREAM[..2], 3),
            (&[130][..], 4),
            (&[192, 0][..], 2),
        ] {
            assert!(expand(stream, length).is_err(), "{stream:?} to {length}");
        }
        assert!(expand(&[255], MAX_MEMBER_BYTES + 1).is_err());
    }

    #[test]
    fn a_directory_both_sizes_divide_is_read_by_what_its_names_say() {
        // Nine Quake entries and eight Daikatana ones are both 576 bytes.
        let directory = |size: usize, count: usize| {
            let mut out = Vec::new();
            for index in 0..count {
                let mut entry = vec![0u8; size];
                entry[..5].copy_from_slice(format!("file{index}").as_bytes());
                entry[56..60].copy_from_slice(&0x0101_0101i32.to_le_bytes());
                if size == DAIKATANA_ENTRY {
                    entry[64..68].copy_from_slice(&0x0102_0304i32.to_le_bytes());
                    entry[68] = 1;
                }
                out.extend(entry);
            }
            out
        };
        assert_eq!(entry_size(&directory(QUAKE_ENTRY, 9)), QUAKE_ENTRY);
        assert_eq!(entry_size(&directory(DAIKATANA_ENTRY, 8)), DAIKATANA_ENTRY);
        assert_eq!(entry_size(&[0; 576]), QUAKE_ENTRY);
        assert_eq!(entry_size(&[0; 72]), DAIKATANA_ENTRY);
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
            assert!(pak.read_file("progs/armor.mdl").is_none());
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
