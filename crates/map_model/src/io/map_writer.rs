//! Writing `.map` files.
//!
//! Writes standard plain-text `.map` files readable by compatible editors and Quake-lineage
//! compilers.
//!
//! Coordinates are written in the editor's own frame — Quake convention, Z up,
//! right-handed. Converting to an engine's frame is the engine importer's job, not the
//! writer's, so that a file saved here and loaded again is unchanged.

use std::fmt::Write;

use geometry_math::vec::{Vec2f, Vec3d};

use super::map_reader::{Brush, Entity, MapFile, Patch, METADATA_MARKER};
use crate::brush_face::{BrushFace, FaceColor};
use crate::map_format::MapFormat;
use crate::uv::UvCoordSystem;

#[path = "primitive_writer.rs"]
mod primitive;
pub use primitive::{write_doom3_map, write_quake3_map};

/// Writes a `.map` file.
pub fn write_map(map: &MapFile) -> String {
    let mut out = String::new();

    if let Some(game) = &map.game {
        let _ = writeln!(out, "// Game: {game}");
    }
    let _ = writeln!(out, "// Format: {}", map.format.name());
    // What version of the editor's own notes -- the ids and the colours -- are written
    // beside the dialect. A reader that meets a higher number knows the file holds
    // something it does not understand.
    let has_multicolor = map
        .entities
        .iter()
        .flat_map(|e| &e.brushes)
        .flat_map(|b| &b.faces)
        .any(|face| matches!(face.paint(), Some(FaceColor::MultiGradient { .. })));
    let has_pending_uv = map
        .entities
        .iter()
        .flat_map(|e| &e.brushes)
        .flat_map(|b| &b.faces)
        .any(|face| face.primitive_uv.is_some());
    let required_version = if has_pending_uv {
        3
    } else if has_multicolor {
        2
    } else {
        0
    };
    let version = if required_version != 0 {
        Some(map.notes_version.unwrap_or(0).max(required_version))
    } else {
        map.notes_version
    };
    if let Some(version) = version {
        let _ = writeln!(out, "// {METADATA_MARKER} format {version}");
    }

    for (index, entity) in map.entities.iter().enumerate() {
        write_entity(&mut out, entity, map.format, index);
    }
    out
}

fn write_entity(out: &mut String, entity: &Entity, format: MapFormat, index: usize) {
    write_node_id(out, entity.id);
    let _ = writeln!(out, "// entity {index}");
    let _ = writeln!(out, "{{");

    // The `mapversion` property identifies the Valve 220 dialect, and belongs on the
    // worldspawn. It is written only if the entity does not already carry it, so a file
    // that had one keeps its own. A dialect without one drops a carried one, which would
    // claim the file is Valve 220; Hexen II's engine also prints "'mapversion' is not a
    // field" for it on every load.
    let has_version = format.map_version().is_some();
    if index == 0 && entity.property("mapversion").is_none() {
        if let Some(version) = format.map_version() {
            let _ = writeln!(out, "\"mapversion\" \"{version}\"");
        }
    }

    for (key, value) in &entity.properties {
        if index == 0 && !has_version && key == "mapversion" {
            continue;
        }
        let _ = writeln!(
            out,
            "\"{}\" \"{}\"",
            escape(key, true),
            escape(value, false)
        );
    }

    for (brush_index, brush) in entity.brushes.iter().enumerate() {
        write_brush(out, brush, format, brush_index);
    }
    for patch in &entity.patches {
        write_patch(out, patch);
    }

    let _ = writeln!(out, "}}");
}

/// Writes what a brush or patch is, on its own comment line before the block.
///
/// A comment, so every other tool in the lineage reads the map and ignores it. Without
/// one a three-way merge cannot tell an edited brush from one removed and one added, and
/// has to guess from how much of the geometry still lines up. The value is a
/// [`NodeId`](crate::node::NodeId): 64 bits whose high end is a random per-process
/// prefix, so a brush one person adds offline and a brush the other adds offline are
/// never the same brush by accident.
fn write_node_id(out: &mut String, id: Option<u64>) {
    if let Some(id) = id {
        let _ = writeln!(out, "// {METADATA_MARKER} id {id:x}");
    }
}

fn write_brush(out: &mut String, brush: &Brush, format: MapFormat, index: usize) {
    write_node_id(out, brush.id);
    // One colour for the whole brush goes on a line of its own before it, and the faces
    // then say nothing. A brush whose faces disagree keeps a comment per face.
    let shared = uniform_color(brush);
    if let Some(color) = shared {
        let _ = write!(out, "// ");
        write_color(out, color);
        let _ = writeln!(out);
    }
    let _ = writeln!(out, "// brush {index}");
    let _ = writeln!(out, "{{");
    for face in &brush.faces {
        write_face(out, face, format, shared.is_none());
    }
    let _ = writeln!(out, "}}");
}

fn write_face(out: &mut String, face: &BrushFace, format: MapFormat, with_color: bool) {
    for point in &face.points {
        let _ = write!(out, "{} ", Point(*point));
    }
    let _ = write!(out, "{}", face.material_name);

    // A paraxial face in a dialect with axes on every line, which is what Valve 220 is
    // to every compiler that reads it: hlcsg stops at the first face line without a `[`.
    let paraxial_with_axes =
        format.has_explicit_uv_axes() && matches!(face.uv_coord_system, UvCoordSystem::Paraxial);
    if format.has_explicit_uv_axes() {
        // A paraxial face writes the axes it maps with, its rotation already applied,
        // which give the same texture coordinates. A note says the face was paraxial, so
        // the editor reads it back as it was; other tools ignore the comment.
        let (u_axis, v_axis) = face
            .uv_coord_system
            .axes(face.normal(), &face.uv_attributes);
        let _ = write!(
            out,
            " [ {} {} ] [ {} {} ]",
            Axis(u_axis),
            UvScalar(face.uv_attributes.offset.x()),
            Axis(v_axis),
            UvScalar(face.uv_attributes.offset.y()),
        );
    } else {
        // The format has no axes to write, so the offsets go out on their own, as the
        // Standard format wants.
        let _ = write!(
            out,
            " {} {}",
            UvScalar(face.uv_attributes.offset.x()),
            UvScalar(face.uv_attributes.offset.y()),
        );
    }

    let _ = write!(
        out,
        " {} {} {}",
        UvScalar(face.uv_attributes.rotation),
        UvScalar(face.uv_attributes.scale.x()),
        UvScalar(face.uv_attributes.scale.y()),
    );

    if format == MapFormat::Hexen2 {
        // Hexen II's extra number, which Raven's qbsp requires on every face. It has no
        // room for the Quake 2 trio.
        let _ = write!(out, " 0");
    } else if let Some(surface) = &face.surface_attributes {
        let _ = write!(
            out,
            " {} {} {}",
            surface.content_flags, surface.surface_flags, surface.value
        );
    } else if format.writes_empty_surface_attributes() {
        // A face that never had flags still writes the trio in Quake 3's dialect, as
        // TrenchBroom does. Neverball's mapc reads a face line only if it is there, and
        // leaves out the whole face otherwise. Quake 2's leaves it out, since there
        // `0 0 0` would replace the flags the compiler takes from the texture.
        let _ = write!(out, " 0 0 0");
    }

    write_face_notes(out, face, with_color, true, paraxial_with_axes);
}

/// Writes the editor's notes on a face as one comment at the end of its line.
///
/// `paraxial` marks a face whose axes were written only because the dialect needs them.
pub(super) fn write_face_notes(
    out: &mut String,
    face: &BrushFace,
    with_color: bool,
    with_pending_uv: bool,
    paraxial: bool,
) {
    if with_color {
        write_face_color(out, face);
    }
    // The colour, when there is one, has opened the comment with the marker already.
    let mut opened = with_color && face.color.is_some();
    let mut open = |out: &mut String| {
        if !opened {
            let _ = write!(out, " // {METADATA_MARKER}");
            opened = true;
        }
    };
    if paraxial {
        open(out);
        let _ = write!(out, " paraxial");
    }
    if let Some(surface) = face.surface_material {
        open(out);
        let _ = write!(
            out,
            " pbr {} {} {}",
            surface.roughness.min(100),
            surface.metallic.min(100),
            surface.emission.min(800)
        );
    }
    if face.export_hidden {
        open(out);
        let _ = write!(out, " export_hidden");
    }
    if let Some(matrix) = face.primitive_uv.as_ref().filter(|_| with_pending_uv) {
        open(out);
        let _ = write!(out, " primitive_uv");
        for value in matrix.iter().flatten() {
            let _ = write!(out, " {}", Scalar(*value));
        }
    }
    let _ = writeln!(out);
}

/// Writes the colour painted on a face, as a comment on the face's own line.
///
/// The `.map` grammar has nowhere to put a colour, and inventing a field would stop every
/// other tool reading the file, so it rides along as a comment with the editor's marker on
/// it. `map_reader::parse_face_color` reads it back; anything else ignores it.
///
/// Nothing here when the brush wrote one colour for all six of its faces; see
/// [`uniform_color`].
fn write_face_color(out: &mut String, face: &BrushFace) {
    let Some(color) = face.paint() else {
        return;
    };
    let _ = write!(out, " // ");
    write_color(out, color);
}

/// The colour every face of the brush shares, or `None` where they do not all share one.
///
/// A brush painted in the editor is painted all over -- the colour panel paints what is
/// selected -- so this is the ordinary case and not a special one: on the dragon in
/// `assets/`, every one of its ninety-nine brushes is uniform, and none is mixed. Writing
/// the colour once rather than on every face is what makes recolouring a brush a one-line
/// change in the file instead of one line per face.
fn uniform_color(brush: &Brush) -> Option<FaceColor> {
    let first = brush.faces.first()?.paint()?;
    brush
        .faces
        .iter()
        .all(|face| face.paint() == Some(first))
        .then_some(first)
}

/// Writes the body of a colour comment: what follows the `//`, and nothing else.
fn write_color(out: &mut String, color: FaceColor) {
    let marker = METADATA_MARKER;
    match color {
        FaceColor::MultiGradient {
            stops,
            radial,
            angle,
            ends,
            ..
        } => {
            let kind = if radial { "radial" } else { "linear" };
            let _ = write!(
                out,
                "{marker} {kind} {} {}",
                Scalar(f64::from(angle)),
                stops.as_slice().len()
            );
            for stop in stops.as_slice() {
                let _ = write!(
                    out,
                    " {} {} {} {}",
                    Scalar(f64::from(stop.position)),
                    stop.color.r,
                    stop.color.g,
                    stop.color.b
                );
            }
            if let Some(ends) = ends {
                let [a, b, c, d] = [ends.start[0], ends.start[1], ends.end[0], ends.end[1]]
                    .map(|v| Scalar(f64::from(v)));
                let _ = write!(out, " ends {a} {b} {c} {d}");
            }
        }
        FaceColor::Solid { color, .. } => {
            let _ = write!(out, "{marker} color {} {} {}", color.r, color.g, color.b);
        }
        FaceColor::Gradient {
            from, to, angle, ..
        } => {
            let _ = write!(
                out,
                "{marker} gradient {} {} {} {} {} {} {}",
                from.r,
                from.g,
                from.b,
                to.r,
                to.g,
                to.b,
                Scalar(f64::from(angle)),
            );
        }
    }
    // The alpha rides at the end so an opaque colour writes exactly as it did before.
    if color.alpha() != 255 {
        let _ = write!(out, " {}", color.alpha());
    }
}

fn write_patch(out: &mut String, patch: &Patch) {
    write_patch_material(out, patch, &patch.material_name);
}

fn write_patch_material(out: &mut String, patch: &Patch, material: &str) {
    write_node_id(out, patch.id);
    let _ = writeln!(out, "{{");
    let _ = writeln!(out, "patchDef2");
    let _ = writeln!(out, "{{");
    let _ = writeln!(out, "{}", material);
    let _ = writeln!(out, "( {} {} 0 0 0 )", patch.columns, patch.rows);

    // The file lays the grid out column by column, the transpose of how it is stored.
    let _ = write!(out, "(");
    for column in 0..patch.columns {
        let _ = write!(out, "\n( ");
        for row in 0..patch.rows {
            let point = patch.control_points[row * patch.columns + column];
            // Five numbers in one pair of brackets, `( x y z u v )`, which is what the
            // format says and what `MapFileSerializer` writes. Reusing `Point` here put
            // the position in brackets of its own -- `( ( x y z ) u v )` -- which this
            // reader is lenient enough to take back, so a map went out and came in again
            // unharmed and nothing said otherwise. No other tool would have read it.
            let _ = write!(
                out,
                "( {} {} {} {} ) ",
                Coordinate(point.position.x()),
                Coordinate(point.position.y()),
                Coordinate(point.position.z()),
                Uv(point.uv)
            );
        }
        let _ = write!(out, ")");
    }
    let _ = writeln!(out, "\n)");

    let _ = writeln!(out, "}}");
    let _ = writeln!(out, "}}");
}

/// A number written the way a `.map` file expects it.
///
/// A [`Display`] rather than a function returning a `String`, because a face is written
/// with fourteen of these and a map has a hundred thousand faces: as `String`s that was
/// most of a million short-lived allocations per save, all of them copied into the
/// output buffer and dropped. This one formats straight into the buffer.
///
/// [`Display`]: std::fmt::Display
struct Scalar(f64);

impl std::fmt::Display for Scalar {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Whole numbers are written without a decimal point, since map coordinates are
        // usually on an integer grid and a file full of `-512.0` reads badly and diffs
        // badly. Anything else keeps enough digits to survive a round trip.
        if self.0 == 0.0 {
            // Keep a negative zero from being written as `-0`.
            return out.write_str("0");
        }
        if self.0 == self.0.trunc() && self.0.abs() < 1e15 {
            return write!(out, "{}", self.0 as i64);
        }
        write!(out, "{}", self.0)
    }
}

/// A UV offset, rotation or scale, which the editor keeps as `f32`: written in the
/// fewest digits that read back as the same `f32`, so a `0.948473` read from a file is
/// not saved again as `0.9484729766845703`.
struct UvScalar(f32);

impl std::fmt::Display for UvScalar {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == self.0.trunc() {
            return Scalar(f64::from(self.0)).fmt(out);
        }
        write!(out, "{}", self.0)
    }
}

/// Remove intersection noise close to the finest editor grid, only for geometry.
/// UVs, angles and other scalar properties retain their full precision.
struct Coordinate(f64);
impl std::fmt::Display for Coordinate {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let nearest = (self.0 * 8.0).round() / 8.0;
        let value = if self.0.is_finite() && (self.0 - nearest).abs() <= 1e-9 {
            nearest
        } else {
            self.0
        };
        Scalar(value).fmt(out)
    }
}

/// A point written as `( x y z )`. See `Scalar` for why this is a [`Display`].
///
/// [`Display`]: std::fmt::Display
struct Point(Vec3d);

impl std::fmt::Display for Point {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "( {} {} {} )",
            Coordinate(self.0.x()),
            Coordinate(self.0.y()),
            Coordinate(self.0.z())
        )
    }
}

/// A UV axis written as `x y z`, without the offset that follows it.
struct Axis(Vec3d);

impl std::fmt::Display for Axis {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "{} {} {}",
            Scalar(self.0.x()),
            Scalar(self.0.y()),
            Scalar(self.0.z())
        )
    }
}

/// A patch control point's UV pair, written as `x y`.
struct Uv(Vec2f);

impl std::fmt::Display for Uv {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "{} {}",
            Scalar(f64::from(self.0.x())),
            Scalar(f64::from(self.0.y()))
        )
    }
}

/// Formats a number the way a `.map` file expects it.
///
/// The `String`-returning form of `Scalar`, kept for callers outside the writer.
pub fn format_scalar(value: f64) -> String {
    Scalar(value).to_string()
}

/// Escapes a property key or value for writing between double quotes.
///
/// A quote is escaped, and so is a backslash in front of a quote or another backslash,
/// which the reader would otherwise take for an escape. Any other backslash goes out as
/// it is: compilers and games read the value literally, and Neverball breaks a message's
/// lines with `\`, so `a\b` written as `a\\b` shows an empty line between the two. A
/// value's last backslash stays single too, since the reader takes a `\"` that ends the
/// line as the closing quote; a key is followed by its value on the same line, so its
/// last backslash is escaped.
fn escape(value: &str, is_key: bool) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => {
                let escaped = match chars.peek() {
                    Some('"' | '\\') => true,
                    None => is_key,
                    Some(_) => false,
                };
                out.push_str(if escaped { "\\\\" } else { "\\" });
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod coordinate_tests {
    use super::*;
    #[test]
    fn geometry_noise_is_removed_without_rounding_uv_or_real_detail() {
        for (value, expected) in [
            (7.9999999999990905, "8"),
            (-32.00000000000056, "-32"),
            (7.8749999999999, "7.875"),
            (-0.0, "0"),
        ] {
            assert_eq!(Coordinate(value).to_string(), expected);
        }
        for value in [7.999999, 0.123456789, 8.00000001] {
            assert_eq!(Coordinate(value).to_string().parse::<f64>().unwrap(), value);
        }
        let uv = 7.9999999999990905;
        assert_eq!(Scalar(uv).to_string().parse::<f64>().unwrap(), uv);
    }

    #[test]
    fn uv_values_keep_the_digits_the_file_gave_them() {
        // LibreQuake's lq_e0m1.map, written by TrenchBroom: an f32 read from it used to
        // come back out as 0.9484729766845703 and 142.6842041015625.
        for (value, expected) in [
            (0.948473_f32, "0.948473"),
            (142.6842, "142.6842"),
            (-0.0, "0"),
            (16.0, "16"),
            (0.5, "0.5"),
        ] {
            assert_eq!(UvScalar(value).to_string(), expected);
        }
        let value = 1.0_f32 / 3.0;
        let written = UvScalar(value).to_string();
        assert_eq!(written.parse::<f32>().expect("a number"), value);
    }
}
