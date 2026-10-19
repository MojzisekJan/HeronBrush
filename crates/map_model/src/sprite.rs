//! Reading the `.spr` sprite an FGD's `iconsprite(...)` names, for drawing a point entity
//! as a little picture rather than as a box.
//!
//! Both the Quake and the Half-Life shape of the `IDSP` file are read. Only the first
//! frame is decoded -- an editor draws a fixed icon, not the animation -- into the RGBA a
//! material holds. A Quake sprite (version 1) has no palette of its own and is coloured
//! with the game's, the way a `.mdl` skin is; a Half-Life sprite (version 2) carries its
//! own palette, and its last colour is taken as transparent.

/// A sprite's first frame, as RGBA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite {
    /// The frame's width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, RGBA, row-major from the top.
    pub pixels: Vec<u8>,
}

/// Why a `.spr` could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpriteError {
    /// The file does not begin with `IDSP`.
    NotASprite,
    /// A version this reader does not know -- only 1 (Quake) and 2 (Half-Life).
    UnknownVersion(i32),
    /// The file ends in the middle of something it was reading.
    Truncated,
    /// The header says there are no frames.
    NoFrames,
}

impl std::fmt::Display for SpriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpriteError::NotASprite => f.write_str("not an IDSP sprite"),
            SpriteError::UnknownVersion(v) => write!(f, "unknown sprite version {v}"),
            SpriteError::Truncated => f.write_str("the sprite file is truncated"),
            SpriteError::NoFrames => f.write_str("the sprite has no frames"),
        }
    }
}

impl std::error::Error for SpriteError {}

/// Reads a `.spr` and decodes its first frame. `game_palette` is 256 RGB triples, needed
/// only for a Quake (version 1) sprite; a Half-Life one carries its own.
pub fn read(bytes: &[u8], game_palette: Option<&[u8]>) -> Result<Sprite, SpriteError> {
    let mut at = Reader::new(bytes);
    if at.take(4)? != b"IDSP" {
        return Err(SpriteError::NotASprite);
    }
    let version = at.int()?;
    if version != 1 && version != 2 {
        return Err(SpriteError::UnknownVersion(version));
    }

    let _type = at.int()?;
    if version == 2 {
        let _tex_format = at.int()?;
    }
    let _bounding_radius = at.float()?;
    let _max_width = at.int()?;
    let _max_height = at.int()?;
    let num_frames = at.int()?;
    let _beam_length = at.float()?;
    let _sync_type = at.int()?;

    if num_frames <= 0 {
        return Err(SpriteError::NoFrames);
    }

    // A version-2 file carries its palette here; a version-1 file uses the game's.
    let embedded_palette: Option<Vec<u8>> = if version == 2 {
        let count = at.short()?.max(0) as usize;
        Some(at.take(count * 3)?.to_vec())
    } else {
        None
    };

    // The first frame. `spritegroup` is 0 for a lone frame; a group has an interval table
    // this reader steps past to reach its first picture.
    let group = at.int()?;
    if group != 0 {
        let count = at.int()?.max(0) as usize;
        let _ = at.take(count * 4)?; // one float interval per frame in the group
    }

    let _origin_x = at.int()?;
    let _origin_y = at.int()?;
    let width = at.int()?.max(0) as usize;
    let height = at.int()?.max(0) as usize;
    let indices = at.take(width * height)?;

    let palette = embedded_palette.as_deref().or(game_palette);
    let transparent_index = if version == 2 { Some(255u8) } else { None };
    let pixels = colour(indices, palette, transparent_index);

    Ok(Sprite {
        width: width as u32,
        height: height as u32,
        pixels,
    })
}

/// Turns palette indices into RGBA. Without a palette, the index is used as a grey so the
/// sprite is at least visible.
fn colour(indices: &[u8], palette: Option<&[u8]>, transparent: Option<u8>) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(indices.len() * 4);
    match palette {
        Some(palette) if palette.len() >= 768 => {
            for &index in indices {
                let alpha = if transparent == Some(index) { 0 } else { 255 };
                let at = index as usize * 3;
                rgba.extend_from_slice(&[palette[at], palette[at + 1], palette[at + 2], alpha]);
            }
        }
        _ => {
            for &index in indices {
                let alpha = if transparent == Some(index) { 0 } else { 255 };
                rgba.extend_from_slice(&[index, index, index, alpha]);
            }
        }
    }
    rgba
}

/// A cursor over the bytes.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], SpriteError> {
        let end = self.at.checked_add(count).ok_or(SpriteError::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(SpriteError::Truncated)?;
        self.at = end;
        Ok(slice)
    }

    fn int(&mut self) -> Result<i32, SpriteError> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn short(&mut self) -> Result<i16, SpriteError> {
        let b = self.take(2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    fn float(&mut self) -> Result<f64, SpriteError> {
        let b = self.take(4)?;
        Ok(f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2x1 sprite in the given version's layout, with one lone frame.
    fn sprite(version: i32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"IDSP");
        out.extend_from_slice(&version.to_le_bytes());
        out.extend_from_slice(&2i32.to_le_bytes()); // type: VP_PARALLEL
        if version == 2 {
            out.extend_from_slice(&3i32.to_le_bytes()); // texFormat: SPR_ALPHTEST
        }
        out.extend_from_slice(&1.0f32.to_le_bytes()); // bounding radius
        out.extend_from_slice(&2i32.to_le_bytes()); // max width
        out.extend_from_slice(&1i32.to_le_bytes()); // max height
        out.extend_from_slice(&1i32.to_le_bytes()); // frames
        out.extend_from_slice(&0.0f32.to_le_bytes()); // beam length
        out.extend_from_slice(&0i32.to_le_bytes()); // sync type
        if version == 2 {
            out.extend_from_slice(&256i16.to_le_bytes()); // palette size
            let mut palette = vec![0u8; 768];
            palette[3] = 40; // colour 1 = dark red
            palette[4] = 50;
            palette[5] = 60;
            out.extend_from_slice(&palette);
        }
        out.extend_from_slice(&0i32.to_le_bytes()); // spritegroup: lone frame
        out.extend_from_slice(&0i32.to_le_bytes()); // origin x
        out.extend_from_slice(&0i32.to_le_bytes()); // origin y
        out.extend_from_slice(&2i32.to_le_bytes()); // width
        out.extend_from_slice(&1i32.to_le_bytes()); // height
        out.extend_from_slice(&[1u8, 255]); // two indices: colour 1, then transparent
        out
    }

    #[test]
    fn a_half_life_sprite_uses_its_own_palette_and_last_colour_is_clear() {
        let frame = read(&sprite(2), None).expect("it reads");
        assert_eq!((frame.width, frame.height), (2, 1));
        assert_eq!(&frame.pixels[0..4], &[40, 50, 60, 255], "colour 1");
        assert_eq!(&frame.pixels[4..8], &[0, 0, 0, 0], "index 255 is transparent");
    }

    #[test]
    fn a_quake_sprite_is_coloured_with_the_game_palette() {
        let mut palette = vec![0u8; 768];
        palette[3] = 200; // colour 1 is red
        let frame = read(&sprite(1), Some(&palette)).expect("it reads");
        assert_eq!(&frame.pixels[0..4], &[200, 0, 0, 255]);
        assert_eq!(&frame.pixels[4..8], &[0, 0, 0, 255], "no transparent index in Quake");
    }

    #[test]
    fn something_that_is_not_a_sprite_is_an_error() {
        assert_eq!(read(b"nope", None), Err(SpriteError::NotASprite));
        assert_eq!(read(&sprite(2)[..12], None), Err(SpriteError::Truncated));
        let mut wrong = sprite(1);
        wrong[4] = 9; // version 9
        assert_eq!(read(&wrong, None), Err(SpriteError::UnknownVersion(9)));
    }
}
