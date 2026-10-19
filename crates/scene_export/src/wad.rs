//! Reader for WAD texture packages, the way Quake and Half-Life ship textures.
//!
//! A map that names its textures out of a `.wad` has no loose files to measure,
//! and without the width and height every UV on every face is wrong. WAD3
//! carries a palette per texture, so those come out in colour on their own.
//! WAD2 shares one palette with the whole game, and needs it pointing at.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum WadError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} is not a WAD2 or WAD3 file")]
    NotAWad { path: String },
    #[error("{path} ends in the middle of its directory")]
    Truncated { path: String },
}

/// One texture inside a package.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Where the miptex starts in the file.
    offset: usize,
    /// `D` in WAD2, `C` in WAD3: the two spellings of "this is a texture".
    kind: u8,
}

#[derive(Debug)]
pub struct Wad {
    pub path: PathBuf,
    bytes: Vec<u8>,
    entries: Vec<Entry>,
    by_name: HashMap<String, usize>,
}

impl Wad {
    pub fn open(path: &Path) -> Result<Wad, WadError> {
        let bytes = std::fs::read(path).map_err(|source| WadError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Wad::from_bytes(path.to_path_buf(), bytes)
    }

    pub fn from_bytes(path: PathBuf, bytes: Vec<u8>) -> Result<Wad, WadError> {
        let name = path.display().to_string();
        if bytes.len() < 12 || (&bytes[0..4] != b"WAD2" && &bytes[0..4] != b"WAD3") {
            return Err(WadError::NotAWad { path: name });
        }
        let count = read_u32(&bytes, 4) as usize;
        let directory = read_u32(&bytes, 8) as usize;
        if directory + count * 32 > bytes.len() {
            return Err(WadError::Truncated { path: name });
        }

        let mut entries = Vec::with_capacity(count);
        let mut by_name = HashMap::new();
        for i in 0..count {
            let at = directory + i * 32;
            let offset = read_u32(&bytes, at) as usize;
            let kind = bytes[at + 12];
            let raw = &bytes[at + 16..at + 32];
            let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
            let name = String::from_utf8_lossy(&raw[..end]).to_string();
            // Only textures. A WAD can also hold sounds, fonts and the console
            // picture, and none of those has a width to read.
            if (kind != b'D' && kind != b'C') || offset + 24 > bytes.len() {
                continue;
            }
            let width = read_u32(&bytes, offset + 16);
            let height = read_u32(&bytes, offset + 20);
            if width == 0 || height == 0 || width > 8192 || height > 8192 {
                continue;
            }
            by_name.insert(name.to_ascii_lowercase(), entries.len());
            entries.push(Entry {
                name,
                width,
                height,
                offset,
                kind,
            });
        }
        Ok(Wad {
            path,
            bytes,
            entries,
            by_name,
        })
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.by_name
            .get(&name.to_ascii_lowercase())
            .map(|i| &self.entries[*i])
    }

    /// The texture as RGBA, or `None` when there is no palette to colour it
    /// with. WAD3 carries its own; WAD2 has to be handed one.
    pub fn pixels(&self, entry: &Entry, fallback: Option<&[u8; 768]>) -> Option<Vec<u8>> {
        let at = entry.offset;
        let first = at + read_u32(&self.bytes, at + 24) as usize;
        let count = (entry.width * entry.height) as usize;
        if first + count > self.bytes.len() {
            return None;
        }
        let indices = &self.bytes[first..first + count];

        let mut own = [0u8; 768];
        let palette: &[u8; 768] = if entry.kind == b'C' {
            // After all four mip levels, two bytes of count, then the palette.
            let mut after = first;
            for level in 0..4 {
                let step = 1 << level;
                after = at
                    + read_u32(&self.bytes, at + 24 + level * 4) as usize
                    + (entry.width as usize / step) * (entry.height as usize / step);
            }
            if after + 2 + 768 > self.bytes.len() {
                return None;
            }
            own.copy_from_slice(&self.bytes[after + 2..after + 2 + 768]);
            &own
        } else {
            fallback?
        };

        // A name beginning with `{` means the last palette entry is a hole,
        // which is how both games did cutouts.
        let cutout = entry.name.starts_with('{');
        let mut out = Vec::with_capacity(count * 4);
        for index in indices {
            let i = *index as usize * 3;
            if cutout && *index == 255 {
                out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                out.extend_from_slice(&[palette[i], palette[i + 1], palette[i + 2], 255]);
            }
        }
        Some(out)
    }
}

/// Every package named by a `wad` key, in the order the key lists them.
///
/// The key holds paths as the mapper's machine saw them, which is rarely where
/// the file is now, so only the file name is used and `look_in` is searched.
pub fn open_all(key: &str, look_in: &[PathBuf]) -> (Vec<Wad>, Vec<String>) {
    let mut wads = Vec::new();
    let mut trouble = Vec::new();
    for named in key.split([';', ',']) {
        let named = named.trim().replace('\\', "/");
        if named.is_empty() {
            continue;
        }
        let file = named.rsplit('/').next().unwrap_or(&named);
        let mut found = false;
        for dir in look_in {
            let path = dir.join(file);
            if !path.is_file() {
                continue;
            }
            match Wad::open(&path) {
                Ok(wad) => {
                    wads.push(wad);
                    found = true;
                }
                Err(e) => trouble.push(e.to_string()),
            }
            break;
        }
        if !found {
            trouble.push(format!(
                "cannot find `{file}`, named in the map's wad key. Point --wad-dir at the folder holding it"
            ));
        }
    }
    (wads, trouble)
}

/// A 256 colour palette out of a `palette.lmp`, which is 768 raw bytes.
pub fn read_palette(path: &Path) -> Result<[u8; 768], WadError> {
    let bytes = std::fs::read(path).map_err(|source| WadError::Read {
        path: path.display().to_string(),
        source,
    })?;
    if bytes.len() < 768 {
        return Err(WadError::Truncated {
            path: path.display().to_string(),
        });
    }
    let mut palette = [0u8; 768];
    palette.copy_from_slice(&bytes[..768]);
    Ok(palette)
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    if at + 4 > bytes.len() {
        return 0;
    }
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a package in memory, so the test needs no file anybody shipped.
    fn make_wad(
        magic: &[u8; 4],
        name: &str,
        width: u32,
        height: u32,
        own_palette: bool,
    ) -> Vec<u8> {
        let mut mip: Vec<u8> = Vec::new();
        let mut raw = [0u8; 16];
        raw[..name.len()].copy_from_slice(name.as_bytes());
        mip.extend_from_slice(&raw);
        mip.extend_from_slice(&width.to_le_bytes());
        mip.extend_from_slice(&height.to_le_bytes());
        let header = 16 + 4 + 4 + 16;
        let mut at = header as u32;
        let mut offsets = Vec::new();
        for level in 0..4u32 {
            offsets.push(at);
            at += (width >> level) * (height >> level);
        }
        for o in &offsets {
            mip.extend_from_slice(&o.to_le_bytes());
        }
        for level in 0..4u32 {
            let count = ((width >> level) * (height >> level)) as usize;
            // Every pixel palette entry 1, except the first, which is 255.
            let mut level_bytes = vec![1u8; count];
            level_bytes[0] = 255;
            mip.extend_from_slice(&level_bytes);
        }
        if own_palette {
            mip.extend_from_slice(&256u16.to_le_bytes());
            let mut palette = vec![0u8; 768];
            palette[3] = 10; // colour 1 is (10, 20, 30)
            palette[4] = 20;
            palette[5] = 30;
            mip.extend_from_slice(&palette);
        }

        let mut out = Vec::new();
        out.extend_from_slice(magic);
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&((12 + mip.len()) as u32).to_le_bytes());
        out.extend_from_slice(&mip);
        // The directory: one entry.
        out.extend_from_slice(&12u32.to_le_bytes());
        out.extend_from_slice(&(mip.len() as u32).to_le_bytes());
        out.extend_from_slice(&(mip.len() as u32).to_le_bytes());
        out.push(if own_palette { b'C' } else { b'D' });
        out.push(0);
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&raw);
        out
    }

    #[test]
    fn a_wad2_gives_up_its_sizes() {
        let wad =
            Wad::from_bytes("t.wad".into(), make_wad(b"WAD2", "city4_1", 64, 32, false)).unwrap();
        assert_eq!(wad.entries().len(), 1);
        let entry = wad.get("CITY4_1").expect("names are matched without case");
        assert_eq!((entry.width, entry.height), (64, 32));
    }

    #[test]
    fn a_wad2_needs_a_palette_and_a_wad3_does_not() {
        let two = Wad::from_bytes("t.wad".into(), make_wad(b"WAD2", "wall", 8, 8, false)).unwrap();
        let entry = two.get("wall").unwrap();
        assert!(two.pixels(entry, None).is_none(), "no palette, no colour");

        let mut palette = [0u8; 768];
        palette[3..6].copy_from_slice(&[10, 20, 30]);
        let coloured = two.pixels(entry, Some(&palette)).unwrap();
        assert_eq!(coloured.len(), 8 * 8 * 4);
        assert_eq!(&coloured[4..8], &[10, 20, 30, 255]);

        let three = Wad::from_bytes("t.wad".into(), make_wad(b"WAD3", "wall", 8, 8, true)).unwrap();
        let entry = three.get("wall").unwrap();
        let coloured = three.pixels(entry, None).expect("its own palette");
        assert_eq!(&coloured[4..8], &[10, 20, 30, 255]);
    }

    #[test]
    fn a_name_in_braces_keeps_its_holes() {
        let wad = Wad::from_bytes("t.wad".into(), make_wad(b"WAD3", "{grate", 8, 8, true)).unwrap();
        let entry = wad.get("{grate").unwrap();
        let pixels = wad.pixels(entry, None).unwrap();
        // The first pixel is palette entry 255, which is the hole.
        assert_eq!(&pixels[0..4], &[0, 0, 0, 0]);
        assert_eq!(pixels[7], 255, "and the rest is solid");
    }

    #[test]
    fn anything_that_is_not_a_wad_says_so() {
        let err = Wad::from_bytes("t.wad".into(), b"not a wad at all".to_vec()).unwrap_err();
        assert!(matches!(err, WadError::NotAWad { .. }));
    }
}
