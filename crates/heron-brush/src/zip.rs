//! A zip archive written in memory: how the web build hands over an export, which on the
//! desktop is a folder of files. Each entry is deflated, unless that would not make it
//! smaller (a PNG), in which case it is stored.

use std::io::Write as _;

/// One file of the archive: its path inside it, with `/` between folders, and its bytes.
pub struct Entry {
    pub path: String,
    pub contents: Vec<u8>,
}

/// The archive holding `entries`, in their order.
pub fn write(entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for entry in entries {
        let crc = crc32fast::hash(&entry.contents);
        let deflated = deflate(&entry.contents);
        let (method, data): (u16, &[u8]) = if deflated.len() < entry.contents.len() { (8, &deflated) } else { (0, &entry.contents) };
        let name = entry.path.as_bytes();
        let offset = out.len() as u32;
        // Local file header.
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed: 2.0
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // the name is UTF-8
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // time
        out.extend_from_slice(&0x0021u16.to_le_bytes()); // date: 1980-01-01
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(entry.contents.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // no extra field
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        // Its central directory record.
        directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        directory.extend_from_slice(&20u16.to_le_bytes()); // made by: 2.0
        directory.extend_from_slice(&20u16.to_le_bytes());
        directory.extend_from_slice(&0x0800u16.to_le_bytes());
        directory.extend_from_slice(&method.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0x0021u16.to_le_bytes());
        directory.extend_from_slice(&crc.to_le_bytes());
        directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(entry.contents.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
        directory.extend_from_slice(&[0; 8]); // extra, comment, disk, internal attributes
        directory.extend_from_slice(&0u32.to_le_bytes()); // external attributes
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name);
    }
    let start = out.len() as u32;
    out.extend_from_slice(&directory);
    // End of central directory.
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]); // this disk, the disk the directory starts on
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // no comment
    out
}

/// Raw deflate, as a zip entry holds it.
fn deflate(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    // Writing to a Vec cannot fail.
    let _ = encoder.write_all(bytes);
    encoder.finish().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The archive opens with the system's unzip and gives back what went in.
    #[test]
    fn an_archive_unzips_to_what_went_in() {
        let text = "a level\n".repeat(200).into_bytes();
        let noise: Vec<u8> = (0..300u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
        let archive = write(&[
            Entry { path: "level.tscn".into(), contents: text.clone() },
            Entry { path: "level/mesh 0.bin".into(), contents: noise.clone() },
        ]);
        let Ok(output) = std::process::Command::new("unzip").arg("-v").output() else { return };
        if !output.status.success() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("heron-zip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("export.zip");
        std::fs::write(&file, archive).unwrap();
        let status = std::process::Command::new("unzip").arg("-q").arg(&file).arg("-d").arg(&dir).status().unwrap();
        assert!(status.success());
        assert_eq!(std::fs::read(dir.join("level.tscn")).unwrap(), text);
        assert_eq!(std::fs::read(dir.join("level/mesh 0.bin")).unwrap(), noise);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
