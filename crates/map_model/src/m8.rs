//! Bounded Heretic II M8 texture decoding.
//!
//! An `.m8` is Raven's version of Quake 2's `.wal`: an indexed image with its own
//! 256-colour palette in the header, up to sixteen mips, and the surface flags, content
//! flags and value its faces get by default. The header is 1040 bytes:
//!
//! | at | what |
//! | --- | --- |
//! | 0 | version, 2 |
//! | 4 | name, 32 bytes |
//! | 36, 100, 164 | widths, heights and offsets of the 16 mips |
//! | 228 | the next frame's name, 32 bytes |
//! | 260 | the palette, 768 bytes of RGB |
//! | 1028, 1032, 1036 | surface flags, content flags, value |
use crate::entity_model::Skin;

/// The size of the header, which is where the first mip usually starts.
pub const HEADER_LEN: usize = 1040;

/// What every `.m8` the game reads has in its first word.
const VERSION: u32 = 2;

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(
        bytes[at..at + 4]
            .try_into()
            .expect("a four-byte slice is a u32"),
    )
}

/// The size of the first mip and where its pixels start, checked against the header
/// alone.
pub fn header(bytes: &[u8]) -> Result<(u32, u32, usize), String> {
    if bytes.len() < HEADER_LEN {
        return Err("truncated M8 header".into());
    }
    if word(bytes, 0) != VERSION {
        return Err("not an M8 texture (version is not 2)".into());
    }
    let (width, height) = (word(bytes, 36), word(bytes, 100));
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) > 16_777_216
    {
        return Err("invalid or oversized M8 dimensions".into());
    }
    let offset = word(bytes, 164) as usize;
    if offset < HEADER_LEN {
        return Err("M8 pixels overlap the header".into());
    }
    Ok((width, height, offset))
}

/// The surface flags, content flags and value an M8's header gives the faces that wear
/// it, if it gives them any. Heretic II's own textures nearly all carry a footstep
/// material in bits 24 and 25 of the surface flags.
pub fn surface_defaults(bytes: &[u8]) -> Option<crate::uv::SurfaceAttributes> {
    let word = |at: usize| Some(i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let defaults = crate::uv::SurfaceAttributes {
        surface_flags: word(1028)?,
        content_flags: word(1032)?,
        value: word(1036)?,
    };
    (defaults != crate::uv::SurfaceAttributes::default()).then_some(defaults)
}

/// The first mip through the texture's own palette. Every pixel is opaque: Heretic II
/// draws a wall's index 255 like any other, and see-through faces come from the
/// surface flags.
pub fn read(bytes: &[u8]) -> Result<Skin, String> {
    let (width, height, offset) = header(bytes)?;
    let count = (width * height) as usize;
    let indices = offset
        .checked_add(count)
        .and_then(|end| bytes.get(offset..end))
        .ok_or("truncated M8 mip data")?;
    let palette = &bytes[260..1028];
    let mut pixels = Vec::with_capacity(count * 4);
    for &index in indices {
        let at = index as usize * 3;
        pixels.extend_from_slice(&[palette[at], palette[at + 1], palette[at + 2], 255]);
    }
    Ok(Skin {
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// An 8 x 4 texture whose pixels are index 1, coloured (12, 34, 56).
    pub fn fixture() -> Vec<u8> {
        let mut bytes = vec![0; HEADER_LEN];
        bytes[0..4].copy_from_slice(&2u32.to_le_bytes());
        bytes[4..10].copy_from_slice(b"a/wall");
        bytes[36..40].copy_from_slice(&8u32.to_le_bytes());
        bytes[100..104].copy_from_slice(&4u32.to_le_bytes());
        bytes[164..168].copy_from_slice(&(HEADER_LEN as u32).to_le_bytes());
        bytes[263..266].copy_from_slice(&[12, 34, 56]);
        bytes.extend(vec![1; 32]);
        bytes[HEADER_LEN] = 0;
        bytes
    }

    #[test]
    fn the_texture_s_own_palette_colours_it() {
        let skin = read(&fixture()).expect("the fixture decodes");
        assert_eq!((skin.width, skin.height), (8, 4));
        assert_eq!(&skin.pixels[..8], &[0, 0, 0, 255, 12, 34, 56, 255]);
        assert_eq!(skin.pixels.len(), 8 * 4 * 4);
    }

    #[test]
    fn the_header_s_flags_are_the_faces_defaults() {
        let mut bytes = fixture();
        assert_eq!(surface_defaults(&bytes), None);
        // Stone footsteps and sky, with no contents: Heretic II's general/sky.
        bytes[1028..1032].copy_from_slice(&0x0200_0004i32.to_le_bytes());
        bytes[1032..1036].copy_from_slice(&0x20i32.to_le_bytes());
        bytes[1036..1040].copy_from_slice(&15i32.to_le_bytes());
        assert_eq!(
            surface_defaults(&bytes),
            Some(crate::uv::SurfaceAttributes {
                surface_flags: 0x0200_0004,
                content_flags: 0x20,
                value: 15,
            })
        );
        assert_eq!(surface_defaults(&bytes[..1039]), None);
    }

    #[test]
    fn a_wal_or_a_truncated_file_is_refused() {
        let source = fixture();
        for end in 0..source.len() {
            assert!(read(&source[..end]).is_err(), "{end}");
        }
        for (at, value) in [(0, 0u32), (36, 0), (100, u32::MAX), (164, 99), (164, 5000)] {
            let mut bytes = source.clone();
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
            assert!(read(&bytes).is_err(), "{at}:{value}");
        }
    }
}
