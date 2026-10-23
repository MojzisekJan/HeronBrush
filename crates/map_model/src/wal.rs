//! Bounded Quake II WAL texture decoding with an external RGB palette.
//!
//! Daikatana's `.wal` is another layout of the same thing. It starts with the version
//! byte 3, then the name, three bytes of padding, the size, nine mip offsets, the
//! animation name, the surface and content flags, its own 256-colour palette and the
//! value. Each texture carries its palette, so it needs no `colormap.pcx`.
use crate::entity_model::Skin;

/// The most a `.wal` header can take up: Daikatana's, with its palette.
pub const HEADER_BYTES: usize = 892;

/// Where the numbers sit in a Quake II header and in a Daikatana one.
struct Layout {
    size: usize,
    mips: usize,
    flags: usize,
    value: usize,
    header: usize,
}

const QUAKE2: Layout = Layout {
    size: 32,
    mips: 40,
    flags: 88,
    value: 96,
    header: 100,
};

const DAIKATANA: Layout = Layout {
    size: 36,
    mips: 44,
    flags: 112,
    value: 888,
    header: HEADER_BYTES,
};

/// Where Daikatana keeps a texture's palette.
const DAIKATANA_PALETTE: usize = 120;

/// Whether this is Daikatana's layout. A Quake II `.wal` starts with its name, and no
/// name starts with the byte 3.
pub fn is_daikatana(bytes: &[u8]) -> bool {
    bytes.first() == Some(&3)
}

fn layout(bytes: &[u8]) -> &'static Layout {
    if is_daikatana(bytes) {
        &DAIKATANA
    } else {
        &QUAKE2
    }
}

/// The palette a Daikatana `.wal` carries, or `None` for a Quake II one.
pub fn embedded_palette(bytes: &[u8]) -> Option<[u8; 768]> {
    if !is_daikatana(bytes) {
        return None;
    }
    bytes
        .get(DAIKATANA_PALETTE..DAIKATANA_PALETTE + 768)?
        .try_into()
        .ok()
}

/// Profiles can name an indexed PCX palette or a raw 768-byte RGB palette lump.
pub fn palette(bytes: &[u8], name: &str) -> Result<[u8; 768], String> {
    if std::path::Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pcx"))
    {
        crate::pcx::palette(bytes)
    } else {
        bytes
            .try_into()
            .map_err(|_| "RGB palette must contain exactly 768 bytes".into())
    }
}

/// Dimensions and mip offsets, validated independently of pixel allocation.
pub fn header(bytes: &[u8]) -> Result<(u32, u32, [usize; 4]), String> {
    let layout = layout(bytes);
    if bytes.len() < layout.header {
        return Err("truncated WAL header".into());
    }
    let word = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"));
    let (width, height) = (word(layout.size), word(layout.size + 4));
    if width < 8
        || height < 8
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) > 16_777_216
    {
        return Err("invalid or oversized WAL dimensions".into());
    }
    let mut offsets = [0; 4];
    let mut end = layout.header;
    for (level, slot) in offsets.iter_mut().enumerate() {
        let offset = word(layout.mips + level * 4) as usize;
        if offset < end {
            return Err("overlapping WAL mip ranges".into());
        }
        let count = mip_pixels(is_daikatana(bytes), width, height, level);
        end = offset.checked_add(count).ok_or("WAL mip range overflow")?;
        *slot = offset;
    }
    Ok((width, height, offsets))
}

/// How many pixels mip `level` has. Quake II halves the size each level. Daikatana does
/// too, but its smaller mips start from the largest power of two that fits: a 96 by 64
/// texture's first mip is 32 by 32, as in e1m1's monitors.
fn mip_pixels(daikatana: bool, width: u32, height: u32, level: usize) -> usize {
    let fit = |side: u32| {
        if daikatana && level > 0 {
            1 << side.ilog2()
        } else {
            side
        }
    };
    ((fit(width) >> level) as usize) * ((fit(height) >> level) as usize)
}

/// The surface flags, content flags and value a WAL's header gives the faces that wear
/// it, if it gives them any.
///
/// `qbsp` uses these for a face whose line in the map has no flags of its own, which is
/// how `sky1.wal` makes a face sky and `ltwhite.wal` makes one shine.
pub fn surface_defaults(bytes: &[u8]) -> Option<crate::uv::SurfaceAttributes> {
    let word = |at: usize| Some(i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let layout = layout(bytes);
    let defaults = crate::uv::SurfaceAttributes {
        surface_flags: word(layout.flags)?,
        content_flags: word(layout.flags + 4)?,
        value: word(layout.value)?,
        color: None,
    };
    (defaults != crate::uv::SurfaceAttributes::default()).then_some(defaults)
}

/// Uses the highest-resolution mip; other mips are range-checked, not expanded.
///
/// A Daikatana `.wal` uses its own palette and ignores the one given.
pub fn read(bytes: &[u8], palette: Option<&[u8; 768]>) -> Result<Skin, String> {
    let (width, height, offsets) = header(bytes)?;
    let embedded = embedded_palette(bytes);
    let palette = embedded
        .as_ref()
        .or(palette)
        .ok_or("WAL requires an external palette (pics/colormap.pcx)")?;
    for (level, offset) in offsets.iter().enumerate() {
        let count = mip_pixels(is_daikatana(bytes), width, height, level);
        if offset
            .checked_add(count)
            .is_none_or(|end| end > bytes.len())
        {
            return Err("truncated WAL mip data".into());
        }
    }
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for &index in &bytes[offsets[0]..offsets[0] + (width * height) as usize] {
        let at = index as usize * 3;
        pixels.extend_from_slice(&[
            palette[at],
            palette[at + 1],
            palette[at + 2],
            if index == 255 { 0 } else { 255 },
        ]);
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
    pub fn fixture() -> Vec<u8> {
        let mut bytes = vec![0; 100];
        bytes[32..36].copy_from_slice(&8u32.to_le_bytes());
        bytes[36..40].copy_from_slice(&8u32.to_le_bytes());
        for level in 0..4 {
            let offset = bytes.len() as u32;
            bytes[40 + level * 4..44 + level * 4].copy_from_slice(&offset.to_le_bytes());
            bytes.extend(vec![1; (8 >> level) * (8 >> level)]);
        }
        bytes[100] = 255;
        bytes
    }
    /// An 8 by 8 Daikatana `.wal` whose palette gives index 1 the colour 12 34 56.
    pub fn daikatana_fixture() -> Vec<u8> {
        let mut bytes = vec![0; HEADER_BYTES];
        bytes[0] = 3;
        bytes[1..11].copy_from_slice(b"e1m1/floor");
        bytes[36..40].copy_from_slice(&8u32.to_le_bytes());
        bytes[40..44].copy_from_slice(&8u32.to_le_bytes());
        for level in 0..4 {
            let offset = bytes.len() as u32;
            bytes[44 + level * 4..48 + level * 4].copy_from_slice(&offset.to_le_bytes());
            bytes.extend(vec![1; (8 >> level) * (8 >> level)]);
        }
        bytes[DAIKATANA_PALETTE + 3..DAIKATANA_PALETTE + 6].copy_from_slice(&[12, 34, 56]);
        bytes
    }

    #[test]
    fn a_daikatana_wal_is_drawn_with_its_own_palette() {
        let bytes = daikatana_fixture();
        assert!(is_daikatana(&bytes) && !is_daikatana(&fixture()));
        assert_eq!(header(&bytes).map(|(w, h, _)| (w, h)), Ok((8, 8)));
        // The palette given is not the one used.
        let other = [200; 768];
        for palette in [None, Some(&other)] {
            let skin = read(&bytes, palette).expect("it decodes");
            assert_eq!(&skin.pixels[..4], &[12, 34, 56, 255]);
        }
        assert!(read(&fixture(), None)
            .expect_err("Quake II needs a palette")
            .contains("external palette"));
        // The header is the palette's length longer than Quake II's.
        assert!(header(&bytes[..HEADER_BYTES - 1]).is_err());
    }

    #[test]
    fn a_daikatana_wal_that_is_not_a_power_of_two_has_power_of_two_mips() {
        // e1m1/+0dfmon2.wal: 96 by 64, with mips of 32 by 32, 16 by 16 and 8 by 8.
        let mut bytes = vec![0; HEADER_BYTES];
        bytes[0] = 3;
        bytes[36..40].copy_from_slice(&96u32.to_le_bytes());
        bytes[40..44].copy_from_slice(&64u32.to_le_bytes());
        for (level, size) in [96 * 64, 32 * 32, 16 * 16, 8 * 8].into_iter().enumerate() {
            let offset = bytes.len() as u32;
            bytes[44 + level * 4..48 + level * 4].copy_from_slice(&offset.to_le_bytes());
            bytes.extend(vec![1; size]);
        }
        assert_eq!(header(&bytes).map(|(w, h, _)| (w, h)), Ok((96, 64)));
        assert!(read(&bytes, None).is_ok());
        // Quake II's halving would want a 48 by 32 first mip, which overlaps the next.
        assert_eq!(mip_pixels(false, 96, 64, 1), 48 * 32);
    }

    #[test]
    fn a_daikatana_wal_s_flags_and_value_come_from_its_own_places() {
        let mut bytes = daikatana_fixture();
        assert_eq!(surface_defaults(&bytes), None);
        bytes[112..116].copy_from_slice(&4i32.to_le_bytes());
        bytes[116..120].copy_from_slice(&8i32.to_le_bytes());
        bytes[888..892].copy_from_slice(&600i32.to_le_bytes());
        assert_eq!(
            surface_defaults(&bytes),
            Some(crate::uv::SurfaceAttributes {
                content_flags: 8,
                surface_flags: 4,
                value: 600,
                color: None,
            })
        );
    }

    #[test]
    fn external_palette_and_transparent_index_are_decoded() {
        let mut palette = [0; 768];
        palette[3..6].copy_from_slice(&[12, 34, 56]);
        let skin = read(&fixture(), Some(&palette)).unwrap();
        assert_eq!((skin.width, skin.height), (8, 8));
        assert_eq!(&skin.pixels[..8], &[0, 0, 0, 0, 12, 34, 56, 255]);
    }
    #[test]
    fn the_header_s_flags_are_the_faces_defaults() {
        let mut bytes = fixture();
        assert_eq!(surface_defaults(&bytes), None);
        bytes[88..92].copy_from_slice(&133i32.to_le_bytes());
        bytes[96..100].copy_from_slice(&300i32.to_le_bytes());
        assert_eq!(
            surface_defaults(&bytes),
            Some(crate::uv::SurfaceAttributes {
                content_flags: 0,
                surface_flags: 133,
                value: 300,
                color: None,
            })
        );
        assert_eq!(surface_defaults(&bytes[..99]), None);
    }
    #[test]
    fn malformed_dimensions_offsets_and_all_truncations_are_rejected() {
        let source = fixture();
        let palette = [0; 768];
        for end in 0..source.len() {
            assert!(read(&source[..end], Some(&palette)).is_err(), "{end}");
        }
        for (at, value) in [
            (32, 0u32),
            (36, u32::MAX),
            (40, 99),
            (44, 100),
            (48, u32::MAX),
            (52, 0),
        ] {
            let mut bytes = source.clone();
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
            assert!(read(&bytes, Some(&palette)).is_err(), "{at}:{value}");
        }
    }
}
