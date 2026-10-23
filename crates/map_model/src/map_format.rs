//! Which dialect of the `.map` file format a document uses.
//!
//! Map dialects select how face lines store UV mappings and optional surface flags.

use std::fmt;
use std::str::FromStr;

/// The dialect of a `.map` file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MapFormat {
    /// The original Quake format: `<material> xoff yoff rot xscale yscale`.
    ///
    /// The UV axes are not stored. They are derived from whichever coordinate plane the
    /// face is most nearly parallel to, which is why a face at an angle shows visibly
    /// stretched material.
    #[default]
    Standard,
    /// The Valve 220 format: `<material> [ ux uy uz uoff ] [ vx vy vz voff ] rot xscale yscale`.
    ///
    /// The UV axes are written out, so a face at any angle keeps its mapping. Rotating
    /// the material rotates the axes; the `rot` field is carried along for compatibility
    /// but does not drive the mapping.
    Valve,
    /// Valve 220 UV axes plus the Quake 2 surface flags trio, left out of a face that
    /// has no flags, as in [`MapFormat::Quake2`].
    Quake2Valve,
    /// Standard UV plus the Quake 2 surface flags trio.
    ///
    /// A face with no flags of its own leaves the trio out. Quake 2's compilers then take
    /// the flags from the texture's `.wal`, and that is what makes `sky1` sky and `clip`
    /// clip; written as `0 0 0` it would clear them.
    Quake2,
    /// Standard UV plus the trio, which every face line has, flags or not.
    ///
    /// Neverball's mapc reads a face line only when all three numbers are there and
    /// leaves the face out otherwise.
    Quake3,
    /// Standard UV plus one more number, Hexen II's: `<material> xoff yoff rot xscale yscale value`.
    ///
    /// Raven's `qbsp` reads the number as the brush's light value and stops with "line is
    /// incomplete" when it is missing, so a face line has to end with it. TrenchBroom
    /// writes it as 0 and so does this editor; the reader takes it and lets it go.
    Hexen2,
}

/// What this editor's own additions to the `.map` format are up to.
///
/// The dialect is Valve 220 and says so in its own header line; this is the version of
/// the notes written beside it -- the ids, the colours -- and it exists so that a file
/// can say what wrote it. A reader that meets a higher number knows there is something
/// in the file it does not understand, and can say so instead of quietly dropping it.
///
/// 1: `// heron_brush id`, `// heron_brush color` and `// heron_brush gradient`, per face or once for a brush
/// whose faces agree. See `docs/map-format.md`.
/// 2: `// heron_brush linear` and `// heron_brush radial` with positioned colour stops.
/// 3: `// heron_brush primitive_uv` preserves unresolved normalized texture matrices.
pub const NOTES_VERSION: u32 = 3;

impl MapFormat {
    /// The dialect that can hold everything this editor knows about a face.
    ///
    /// A face's UV axes are the reason there has to be one: the Standard dialect has
    /// nowhere on a face line to put them, so a face that carries its own comes back
    /// deriving them from its dominant plane instead. Valve 220 has the room, and both
    /// the reader and the writer decide per face rather than per file -- the reader by
    /// whether the line has `[ ... ]` axes, the writer by asking the face -- so a map in
    /// this dialect holds faces of both kinds side by side and gives each one back as
    /// what it was.
    ///
    /// That is what lets the editor work without being told a flavour first, and what
    /// lets a map be exported for one engine today and another tomorrow with nothing
    /// lost in between. `map_model/tests/roundtrip.rs` holds it to both claims.
    pub const LOSSLESS: MapFormat = MapFormat::Valve;

    /// Returns whether faces in this format store their UV axes explicitly.
    #[inline]
    pub fn has_explicit_uv_axes(self) -> bool {
        matches!(self, MapFormat::Valve | MapFormat::Quake2Valve)
    }

    /// Returns whether faces in this format may carry the surface flags trio.
    #[inline]
    pub fn has_surface_attributes(self) -> bool {
        matches!(
            self,
            MapFormat::Quake2 | MapFormat::Quake2Valve | MapFormat::Quake3
        )
    }

    /// Returns whether a face without flags still writes the trio, as `0 0 0`.
    #[inline]
    pub fn writes_empty_surface_attributes(self) -> bool {
        self == MapFormat::Quake3
    }

    /// Returns the name written into the `// Format:` header comment.
    #[inline]
    pub fn name(self) -> &'static str {
        match self {
            MapFormat::Standard => "Standard",
            MapFormat::Valve => "Valve",
            MapFormat::Quake2 => "Quake2",
            MapFormat::Quake2Valve => "Quake2 (Valve)",
            MapFormat::Quake3 => "Quake3",
            MapFormat::Hexen2 => "Hexen2",
        }
    }

    /// Returns the value written for the `mapversion` worldspawn property, if the format
    /// calls for one.
    ///
    /// The Valve 220 dialect is identified by `"mapversion" "220"`; the others leave the
    /// property out.
    #[inline]
    pub fn map_version(self) -> Option<u32> {
        self.has_explicit_uv_axes().then_some(220)
    }
}

impl fmt::Display for MapFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The error returned when a format name is not recognized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownMapFormat(pub String);

impl fmt::Display for UnknownMapFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown map format \"{}\"", self.0)
    }
}

impl std::error::Error for UnknownMapFormat {}

impl FromStr for MapFormat {
    type Err = UnknownMapFormat;

    /// Parses the name from a `// Format:` header comment.
    ///
    /// Matching is case-insensitive, since the header is written by hand as often as by a
    /// tool.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "standard" => Ok(MapFormat::Standard),
            "valve" => Ok(MapFormat::Valve),
            // Quake 3 and Daikatana share Quake 2's face grammar for a non-primitive
            // brush; a `brushDef` primitive is skipped whatever the header says.
            "quake2" | "daikatana" => Ok(MapFormat::Quake2),
            "quake3" | "quake3 (legacy)" => Ok(MapFormat::Quake3),
            "quake2 (valve)" | "quake2_valve" | "quake3 (valve)" | "quake3_valve" => {
                Ok(MapFormat::Quake2Valve)
            }
            "hexen2" => Ok(MapFormat::Hexen2),
            _ => Err(UnknownMapFormat(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_name_is_read_case_insensitively_with_the_known_aliases() {
        for format in [
            MapFormat::Standard,
            MapFormat::Valve,
            MapFormat::Quake2,
            MapFormat::Quake2Valve,
            MapFormat::Quake3,
            MapFormat::Hexen2,
        ] {
            assert_eq!(format.name().parse(), Ok(format), "{format}");
        }
        assert_eq!("Valve".parse(), Ok(MapFormat::Valve));
        assert_eq!("STANDARD".parse(), Ok(MapFormat::Standard));
        assert_eq!("Quake2 (Valve)".parse(), Ok(MapFormat::Quake2Valve));
        // Quake 3 and Daikatana share Quake 2's face grammar.
        assert_eq!("Quake3".parse(), Ok(MapFormat::Quake3));
        assert_eq!("Quake3 (legacy)".parse(), Ok(MapFormat::Quake3));
        assert_eq!("quake3 (valve)".parse(), Ok(MapFormat::Quake2Valve));
        assert_eq!("Daikatana".parse(), Ok(MapFormat::Quake2));
        assert_eq!("Hexen2".parse(), Ok(MapFormat::Hexen2));
        assert!("Hexen3".parse::<MapFormat>().is_err());
    }
}
