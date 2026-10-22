//! Bounded decoding of 8-bit, single-plane, RLE PCX images for skins and materials.
use crate::entity_model::Skin;

/// Validates the indexed PCX header without expanding its pixel data.
pub fn dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    let bad = || "invalid or unsupported indexed PCX".to_string();
    if bytes.len() < 128 || bytes[..4] != [10, 5, 1, 8] || bytes[65] != 1 {
        return Err(bad());
    }
    let short = |at| u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize;
    let width = short(8)
        .checked_sub(short(4))
        .and_then(|v| v.checked_add(1))
        .ok_or_else(bad)?;
    let height = short(10)
        .checked_sub(short(6))
        .and_then(|v| v.checked_add(1))
        .ok_or_else(bad)?;
    let stride = short(66);
    if stride < width || width > 8192 || height > 8192 || width * height > 16_777_216 {
        return Err(bad());
    }
    Ok((width as u32, height as u32))
}

/// Reads the trailing RGB palette without expanding image pixels.
pub fn palette(bytes: &[u8]) -> Result<[u8; 768], String> {
    dimensions(bytes)?;
    if bytes.len() < 128 + 769 || bytes[bytes.len() - 769] != 12 {
        return Err("missing indexed PCX palette".into());
    }
    Ok(bytes[bytes.len() - 768..].try_into().unwrap())
}

/// Decodes a version-5 indexed PCX with its trailing 256-colour palette.
/// Index 255 is transparent, following Quake II indexed-image rendering.
pub fn read(bytes: &[u8]) -> Result<Skin, String> {
    let bad = || "invalid or unsupported indexed PCX".to_string();
    let (width, height) = dimensions(bytes)?;
    let (width, height) = (width as usize, height as usize);
    if bytes.len() < 128 + 769 {
        return Err(bad());
    }
    let stride = u16::from_le_bytes([bytes[66], bytes[67]]) as usize;
    let palette_at = bytes.len() - 769;
    if bytes[palette_at] != 12 {
        return Err(bad());
    }
    let palette = &bytes[palette_at + 1..];
    let mut pixels = Vec::with_capacity(width * height * 4);
    let mut at = 128;
    for _ in 0..height {
        let mut column = 0;
        while column < stride {
            let token = *bytes.get(at).filter(|_| at < palette_at).ok_or_else(bad)?;
            at += 1;
            let (count, value) = if token & 0xc0 == 0xc0 {
                let value = *bytes.get(at).filter(|_| at < palette_at).ok_or_else(bad)?;
                at += 1;
                ((token & 0x3f) as usize, value)
            } else {
                (1, token)
            };
            if count == 0 || column + count > stride {
                return Err(bad());
            }
            for _ in column..(column + count).min(width) {
                let i = value as usize * 3;
                pixels.extend_from_slice(&[
                    palette[i],
                    palette[i + 1],
                    palette[i + 2],
                    if value == 255 { 0 } else { 255 },
                ]);
            }
            column += count;
        }
    }
    Ok(Skin {
        width: width as u32,
        height: height as u32,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut b = vec![0; 128];
        b[..4].copy_from_slice(&[10, 5, 1, 8]);
        b[8] = 2;
        b[10] = 1;
        b[65] = 1;
        b[66] = 4;
        b.extend_from_slice(&[0xc3, 1, 0, 2, 1, 2, 0]);
        b.push(12);
        let mut palette = vec![0; 768];
        palette[3..9].copy_from_slice(&[255, 0, 0, 0, 255, 0]);
        b.extend(palette);
        b
    }
    #[test]
    fn rle_rows_padding_and_palette_are_decoded() {
        let skin = read(&fixture()).unwrap();
        assert_eq!((skin.width, skin.height), (3, 2));
        assert_eq!(
            skin.pixels,
            [[255, 0, 0, 255]; 3]
                .concat()
                .into_iter()
                .chain([0, 255, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255])
                .collect::<Vec<_>>()
        );
    }
    #[test]
    fn palette_index_255_is_transparent_for_quake_two_skins() {
        let mut b = fixture();
        b.splice(128..130, [0xc3, 255]);
        let skin = read(&b).unwrap();
        assert_eq!(skin.pixels[3], 0);
        assert_eq!(skin.pixels[7], 0);
        assert_eq!(skin.pixels[15], 255);
    }
    #[test]
    fn truncation_bad_runs_layouts_and_dimensions_are_rejected() {
        let original = fixture();
        for end in 0..original.len() {
            assert!(read(&original[..end]).is_err());
        }
        for (at, value) in [
            (0, 0),
            (1, 3),
            (2, 0),
            (3, 4),
            (65, 3),
            (66, 2),
            (128, 0xc0),
            (128, 0xc5),
        ] {
            let mut b = original.clone();
            b[at] = value;
            assert!(read(&b).is_err(), "{at}:{value}");
        }
        let mut b = original;
        b[8..10].copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(read(&b).is_err());
    }
}
