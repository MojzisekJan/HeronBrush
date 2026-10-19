//! Bounded Quake II WAL texture decoding with an external RGB palette.
use crate::entity_model::Skin;

/// Profiles can name an indexed PCX palette or a raw 768-byte RGB palette lump.
pub fn palette(bytes: &[u8], name: &str) -> Result<[u8;768], String> {
    if std::path::Path::new(name).extension().is_some_and(|ext|ext.eq_ignore_ascii_case("pcx")) {
        crate::pcx::palette(bytes)
    } else {
        bytes.try_into().map_err(|_|"RGB palette must contain exactly 768 bytes".into())
    }
}

/// Dimensions and mip offsets, validated independently of pixel allocation.
pub fn header(bytes: &[u8]) -> Result<(u32, u32, [usize; 4]), String> {
    if bytes.len() < 100 { return Err("truncated WAL header".into()); }
    let word = |at| u32::from_le_bytes(bytes[at..at+4].try_into().unwrap());
    let (width, height) = (word(32), word(36));
    if width < 8 || height < 8 || width > 8192 || height > 8192 || u64::from(width)*u64::from(height) > 16_777_216 {
        return Err("invalid or oversized WAL dimensions".into());
    }
    let mut offsets = [0; 4];
    let mut end = 100;
    for level in 0..4 {
        let offset = word(40+level*4) as usize;
        if offset < end { return Err("overlapping WAL mip ranges".into()); }
        let count = ((width >> level) as usize) * ((height >> level) as usize);
        end = offset.checked_add(count).ok_or("WAL mip range overflow")?;
        offsets[level] = offset;
    }
    Ok((width, height, offsets))
}

/// Uses the highest-resolution mip; other mips are range-checked, not expanded.
pub fn read(bytes: &[u8], palette: &[u8; 768]) -> Result<Skin, String> {
    let (width, height, offsets) = header(bytes)?;
    for (level, offset) in offsets.iter().enumerate() {
        let count = ((width >> level) as usize) * ((height >> level) as usize);
        if offset.checked_add(count).is_none_or(|end| end > bytes.len()) {
            return Err("truncated WAL mip data".into());
        }
    }
    let mut pixels = Vec::with_capacity((width*height*4) as usize);
    for &index in &bytes[offsets[0]..offsets[0]+(width*height) as usize] {
        let at = index as usize*3;
        pixels.extend_from_slice(&[palette[at],palette[at+1],palette[at+2],if index==255 {0} else {255}]);
    }
    Ok(Skin { width, height, pixels })
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
            bytes[40+level*4..44+level*4].copy_from_slice(&offset.to_le_bytes());
            bytes.extend(vec![1; (8>>level)*(8>>level)]);
        }
        bytes[100] = 255;
        bytes
    }
    #[test]
    fn external_palette_and_transparent_index_are_decoded() {
        let mut palette = [0; 768]; palette[3..6].copy_from_slice(&[12,34,56]);
        let skin = read(&fixture(), &palette).unwrap();
        assert_eq!((skin.width,skin.height),(8,8));
        assert_eq!(&skin.pixels[..8], &[0,0,0,0,12,34,56,255]);
    }
    #[test]
    fn malformed_dimensions_offsets_and_all_truncations_are_rejected() {
        let source = fixture(); let palette = [0;768];
        for end in 0..source.len() { assert!(read(&source[..end],&palette).is_err(), "{end}"); }
        for (at, value) in [(32,0u32),(36,u32::MAX),(40,99),(44,100),(48,u32::MAX),(52,0)] {
            let mut bytes=source.clone(); bytes[at..at+4].copy_from_slice(&value.to_le_bytes());
            assert!(read(&bytes,&palette).is_err(), "{at}:{value}");
        }
    }
}
